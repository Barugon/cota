use chrono::NaiveDateTime;
use eframe::egui::{TextStyle, Ui};
use num_format::Locale;
use regex::Regex;
use std::{
  ops::{Range, RangeInclusive},
  sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
  },
};

pub const APP_ICON: &[u8] = include_bytes!("res/icon.png");
pub const APP_NAME: &str = env!("CARGO_PKG_NAME");
pub const APP_TITLE: &str = env!("CARGO_PKG_DESCRIPTION");
pub const APP_AUTHORS: &str = env!("CARGO_PKG_AUTHORS");
pub const APP_VERSION: &str = env!("CARGO_PKG_VERSION");

pub const LEVEL_EXP: [i64; 200] = include!("res/level_exp_values");
pub const SKILL_EXP: [i64; 200] = include!("res/skill_exp_values");
pub const LVL_RANGE: RangeInclusive<i32> = 1..=200;

#[macro_export]
macro_rules! debugln {
  ($($arg:tt)*) => (#[cfg(debug_assertions)] println!($($arg)*));
}

#[macro_export]
macro_rules! ok {
  ($res:expr) => {
    match $res {
      Ok(val) => val,
      Err(_) => {
        debugln!("{}", _err);
        return;
      }
    }
  };
  ($res:expr, $ret:expr) => {
    match $res {
      Ok(val) => val,
      Err(_err) => {
        debugln!("{}", _err);
        return $ret;
      }
    }
  };
}

#[derive(Default)]
struct State {
  /// Show the "progress" cursor.
  busy: AtomicBool,

  /// Disable the main UI.
  disabled: AtomicBool,
}

#[derive(Clone, Default)]
pub struct AppState {
  state: Arc<State>,
}

impl AppState {
  pub fn set_busy(&mut self, busy: bool) {
    self.state.busy.store(busy, Ordering::Relaxed);
  }

  #[must_use]
  pub fn is_busy(&self) -> bool {
    self.state.busy.load(Ordering::Relaxed)
  }

  pub fn set_disabled(&mut self, disable: bool) {
    self.state.disabled.store(disable, Ordering::Relaxed);
  }

  #[must_use]
  pub fn is_disabled(&self) -> bool {
    self.state.disabled.load(Ordering::Relaxed)
  }
}

#[derive(Clone, Default)]
pub struct Cancel {
  canceled: Arc<AtomicBool>,
}

impl Cancel {
  pub fn cancel(&mut self) {
    self.canceled.store(true, Ordering::Relaxed);
  }

  #[must_use]
  pub fn is_canceled(&self) -> bool {
    self.canceled.load(Ordering::Relaxed)
  }
}

fn find_ignore_case(text: &str, find: &str) -> Option<Range<usize>> {
  if text.is_empty() || find.is_empty() {
    return None;
  }

  struct ToCaseNext<I: Iterator> {
    next: usize,
    iter: I,
  }

  impl<I: Iterator> Iterator for ToCaseNext<I> {
    type Item = (usize, I::Item);

    fn next(&mut self) -> Option<Self::Item> {
      Some((self.next, self.iter.next()?))
    }
  }

  // Iterator that returns the byte position of the next character
  // and the current character converted to uppercase.
  let mut text_iter = text.char_indices().flat_map(|(index, ch)| ToCaseNext {
    next: index + ch.len_utf8(),
    iter: ch.to_uppercase(),
  });

  let find = find.to_uppercase();
  let mut find_iter = find.chars();
  let mut start = 0;
  let mut end = 0;

  loop {
    // If we made it to the end of find_iter then it's a match.
    let Some(find_ch) = find_iter.next() else { return Some(start..end) };

    // Exit if we arrive at the end of text_iter.
    let (next, upper_ch) = text_iter.next()?;

    // Set the end to the next character.
    end = next;

    if upper_ch != find_ch {
      // Characters don't match, reset find_iter.
      find_iter = find.chars();

      // Set the start to the next character.
      start = next;
    }
  }
}

#[derive(Clone)]
pub enum Search {
  /// Search for the specified string.
  String {
    find: String,
    ignore_case: bool,
  },
  // Use regular expression for pattern matching.
  Regex(Regex),
}

impl Search {
  pub fn find_in(&self, text: &str) -> Option<Range<usize>> {
    match self {
      Search::String { find, ignore_case } => {
        if *ignore_case {
          return find_ignore_case(text, find);
        } else if let Some(pos) = text.find(find) {
          return Some(pos..pos + find.len());
        }
      }
      Search::Regex(regex) => {
        if let Some(pos) = regex.find(text) {
          return Some(pos.start()..pos.end());
        }
      }
    }
    None
  }
}

#[derive(Clone, Copy, Debug)]
pub enum SkillCategory {
  Adventurer,
  Producer,
}

#[derive(Clone, Default)]
pub struct Requires {
  pub id: u32,
  pub lvl: i32,
}

#[derive(Clone, Default)]
pub struct SkillInfo {
  pub name: &'static str,
  pub mul: f64,
  pub id: u32,
  pub reqs: Vec<Requires>,
}

pub struct SkillInfoGroup {
  pub name: &'static str,
  pub skills: Vec<SkillInfo>,
}

impl SkillInfoGroup {
  fn new(name: &'static str) -> Self {
    Self {
      name,
      skills: Vec::new(),
    }
  }
}

/// Parse the CSV for adventurer or producer skills.
pub fn parse_skill_info_groups(category: SkillCategory) -> Vec<SkillInfoGroup> {
  let text = match category {
    SkillCategory::Adventurer => include_str!("res/adventurer_skills.csv"),
    SkillCategory::Producer => include_str!("res/producer_skills.csv"),
  };
  let mut skill_groups = Vec::new();
  let mut skill_group = SkillInfoGroup::new(Default::default());

  // Temporary vector to hold skill requirements in order to keep memory use and reallocations low.
  let mut tmp_reqs = Vec::new();

  for line in text.lines() {
    let mut fields = line.split(',');
    if let Some(group) = fields.next() {
      if group != skill_group.name {
        if !skill_group.name.is_empty() {
          skill_groups.push(skill_group);
        }
        skill_group = SkillInfoGroup::new(group);
      }

      let name = fields.next().unwrap();
      let mul = fields.next().unwrap().parse().unwrap();
      let id = fields.next().unwrap().parse().unwrap();

      while let Some(id) = fields.next() {
        let id = id.parse().unwrap();
        let lvl = fields.next().unwrap().parse().unwrap();
        tmp_reqs.push(Requires { id, lvl });
      }

      let reqs = tmp_reqs.clone();
      tmp_reqs.clear();

      skill_group.skills.push(SkillInfo {
        name,
        mul,
        id,
        reqs,
      });
    }
  }

  if !skill_group.name.is_empty() {
    skill_groups.push(skill_group);
  }

  skill_groups
}

/// Return the byte distance between `text` and `sub`.
pub fn offset(text: &str, sub: &str) -> Option<usize> {
  let text_addr = text.as_ptr() as usize;
  let sub_addr = sub.as_ptr() as usize;
  if (text_addr..text_addr + text.len()).contains(&sub_addr) {
    return Some(sub_addr - text_addr);
  }
  None
}

/// Get the system's locale.
pub fn get_locale() -> Locale {
  if let Some(name) = sys_locale::get_locale() {
    let name = name.replace('_', "-");
    let names = Locale::available_names();
    let uname = name.to_uppercase();
    let mut uname = uname.as_str();

    loop {
      // Look for a match.
      if let Ok(pos) = names.binary_search_by(|n| n.to_uppercase().as_str().cmp(uname)) {
        if let Ok(locale) = Locale::from_name(names[pos]) {
          return locale;
        }
      }

      // Chop off the end.
      if let Some(pos) = uname.rfind('-') {
        uname = &uname[0..pos];
      } else {
        break;
      }
    }
  }

  Locale::en
}

/// Replace a single occurrence of a comma or arabic decimal with a period.
pub fn replace_decimal(text: &str) -> String {
  const ARABIC_DECIMAL: char = '\u{66b}';
  const COMMA: char = ',';
  const PERIOD: char = '.';

  // The output can never be larger than the input.
  let mut result = String::with_capacity(text.len());
  let mut iter = text.chars();

  for ch in &mut iter {
    match ch {
      COMMA | ARABIC_DECIMAL => {
        result.push(PERIOD);
        break;
      }
      _ => result.push(ch),
    }
  }

  for ch in &mut iter {
    result.push(ch);
  }

  result
}

/// Nicely format a f64 for display.
pub fn f64_to_string(value: f64, locale: Locale) -> String {
  format!("{:.6}", value)
    .trim_end_matches('0')
    .trim_end_matches('.')
    .replacen('.', locale.decimal(), 1)
}

/// Convert a timestamp into a date & time string.
pub fn timestamp_to_string(ts: Option<i64>) -> String {
  let Some(ts) = ts else { return String::new() };
  let Some(dt) = NaiveDateTime::from_timestamp_opt(ts, 0) else { return String::new() };
  dt.format("%Y-%m-%d %H:%M:%S").to_string()
}

/// Get the size (thickness) of a scrollbar.
pub fn scroll_bar_size(ui: &Ui) -> f32 {
  let spacing = ui.spacing();
  spacing.scroll_bar_inner_margin + spacing.scroll_bar_width + spacing.scroll_bar_outer_margin
}

/// Get the size (height) of a Button.
pub fn button_size(ui: &Ui) -> f32 {
  text_size(ui) + ui.spacing().button_padding[1] * 2.0
}

/// Get the size (height) of body text.
pub fn text_size(ui: &Ui) -> f32 {
  TextStyle::Body.resolve(ui.style()).size
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_replace_decimal() {
    assert_eq!("123.4", replace_decimal("123.4"));
    assert_eq!("123.4", replace_decimal("123,4"));
    assert_eq!("123.", replace_decimal("123,"));
    assert_eq!(".4", replace_decimal(",4"));
    assert_eq!("123.4", replace_decimal("123\u{66b}4"));
    assert_eq!("123.", replace_decimal("123\u{66b}"));
    assert_eq!(".4", replace_decimal("\u{66b}4"));
  }

  #[test]
  fn test_find_ignore_case() {
    let text = "Test for 'tschüß' in this text";
    let result = find_ignore_case(text, "TSCHÜSS");
    assert!(result.is_some());

    let range = result.unwrap();
    assert!(range.start == 10);

    let len = "tschüß".len();
    assert!(range.end == range.start + len);

    let text = "Is 'TSCHÜSS' present?";
    let result = find_ignore_case(text, "tschüß");
    assert!(result.is_some());

    let range = result.unwrap();
    assert!(range.start == 4);

    let len = "TSCHÜSS".len();
    assert!(range.end == range.start + len);

    let text = "Find 'ghi\u{307}j'";
    let result = find_ignore_case(text, "ghİj");
    assert!(result.is_some());

    let range = result.unwrap();
    assert!(range.start == 6);

    let len = "ghi\u{307}j".len();
    assert!(range.end == range.start + len);

    let text = "Abc aBc abC";
    let result = find_ignore_case(text, "abc");
    assert!(result.is_some());

    let range = result.unwrap();
    assert!(range.start == 0 && range.end == 3);

    let text = "cbA cBa abC";
    let result = find_ignore_case(text, "abc");
    assert!(result.is_some());

    let range = result.unwrap();
    assert!(range.start == 8 && range.end == 11);
  }
}
