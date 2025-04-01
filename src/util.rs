use chrono::{DateTime, TimeZone, Utc};
use clipboard::{ClipboardContext, ClipboardProvider};
use eframe::{
  egui::{Context, Image, TextStyle, Ui},
  epaint::{ColorImage, TextureHandle, TextureId, Vec2},
};
use num_format::Locale;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::{
  borrow::Cow,
  cell::RefCell,
  mem,
  ops::{Range, RangeInclusive},
  sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
  },
};

pub const APP_ICON: &[u8] = include_bytes!("../res/icon.png");
pub const APP_NAME: &str = env!("CARGO_PKG_NAME");
pub const APP_TITLE: &str = env!("CARGO_PKG_DESCRIPTION");
pub const APP_AUTHORS: &str = env!("CARGO_PKG_AUTHORS");
pub const APP_VERSION: &str = env!("CARGO_PKG_VERSION");

pub const LEVEL_EXP: &[i64] = include!("../res/level_exp_values.rs");
pub const SKILL_EXP: &[i64] = include!("../res/skill_exp_values.rs");
pub const LVL_RANGE: RangeInclusive<i32> = 1..=200;

/// Number of seconds in an hour (one in-game day).
pub const HOUR_SECS: i64 = 60 * 60;

/// Number of seconds in a fortnight (two weeks, one in-game year).
pub const FORTNIGHT_SECS: i64 = HOUR_SECS * 24 * 14;

pub type Error = Cow<'static, str>;

#[macro_export]
macro_rules! debugln {
  ($($arg:tt)*) => (#[cfg(debug_assertions)] println!($($arg)*));
}

/// Return from function (and print error) if `Result` is not `Ok`.
#[macro_export]
macro_rules! ok {
  ($res:expr) => {
    match $res {
      Ok(val) => val,
      Err(err) => {
        println!("{err:?}");
        return;
      }
    }
  };
  ($res:expr, $ret:expr) => {
    match $res {
      Ok(val) => val,
      Err(err) => {
        println!("{err:?}");
        return $ret;
      }
    }
  };
}

/// Print if `Err`.
#[macro_export]
macro_rules! err {
  ($res:expr) => {
    if let Err(err) = $res {
      println!("{err:?}");
    }
  };
}

/// Nicely format a f64 for display.
#[macro_export]
macro_rules! f64_to_string {
  ($value:expr, 2, $locale:expr) => {
    format!("{:.2}", $value)
      .trim_end_matches('0')
      .trim_end_matches('.')
      .replacen('.', $locale.decimal(), 1)
  };
  ($value:expr, 6, $locale:expr) => {
    format!("{:.6}", $value)
      .trim_end_matches('0')
      .trim_end_matches('.')
      .replacen('.', $locale.decimal(), 1)
  };
}

pub struct Picture {
  name: String,
  size: Vec2,
  rgba: RefCell<ColorImage>,
  texture: RefCell<Option<TextureHandle>>,
}

impl Picture {
  pub fn new(name: String, data: &[u8]) -> Self {
    let image = image::load_from_memory(data).unwrap();
    let size = [image.width() as usize, image.height() as usize];
    let pixels = image.into_rgba8();
    let pixels = pixels.as_flat_samples();
    let rgba = ColorImage::from_rgba_unmultiplied(size, pixels.as_slice());
    let size = Vec2::new(size[0] as f32, size[1] as f32);
    Self {
      name,
      size,
      rgba: RefCell::new(rgba),
      texture: RefCell::new(None),
    }
  }

  pub fn size(&self) -> Vec2 {
    self.size
  }

  pub fn texture_id(&self, ctx: &Context) -> TextureId {
    let mut texture = self.texture.borrow_mut();
    if texture.is_none() {
      let rgba = mem::take(&mut *self.rgba.borrow_mut());
      *texture = Some(ctx.load_texture(&self.name, rgba, Default::default()));
    }
    texture.as_ref().unwrap().id()
  }

  pub fn image(&self, ctx: &Context) -> Image {
    Image::new((self.texture_id(ctx), self.size))
  }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum Page {
  Chronometer,
  Experience,
  Farming,
  Offline,
  Stats,
}

/// Get the index of the matching element or the one just below if an exact match is not found.
pub fn floor_search<T: Ord>(value: T, values: &[T]) -> Option<usize> {
  match values.binary_search(&value) {
    Ok(idx) => Some(idx),
    Err(idx) => {
      if idx > 0 {
        Some(idx - 1)
      } else {
        None
      }
    }
  }
}

pub fn set_clipboard_contents(text: String) {
  let mut ctx: ClipboardContext = ok!(ClipboardProvider::new());
  err!(ctx.set_contents(text));
}

/// SotA epoch (date/time of lunar cataclysm).
pub fn get_epoch() -> DateTime<Utc> {
  Utc.with_ymd_and_hms(1997, 9, 2, 0, 0, 0).unwrap()
}

/// Get the remaining time in XXh XXm XXs format.
pub fn get_countdown_text(sec: i64) -> String {
  if sec >= 60 {
    let min = sec / 60;
    let sec = sec - min * 60;
    if min >= 60 {
      let hour = min / 60;
      let min = min - hour * 60;
      return format!("{hour:02}h {min:02}m {sec:02}s");
    }
    return format!("{min:02}m {sec:02}s");
  }
  format!("{sec:02}s")
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
    let Some(find_ch) = find_iter.next() else {
      return Some(start..end);
    };

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
  String { find: String, ignore_case: bool },

  /// Use regular expression for pattern matching.
  Regex(Regex),
}

impl Search {
  pub fn find_in(&self, text: &str) -> Option<Range<usize>> {
    if !text.is_empty() {
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
    }
    None
  }
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
  to_locale(sys_locale::get_locale().as_deref())
}

fn to_locale(name: Option<&str>) -> Locale {
  if let Some(name) = name {
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
  text.replacen([',', '\u{66B}'], ".", 1)
}

/// Remove all digit grouping separators (comma, period, single quote and non-breaking space).
pub fn remove_separators(text: &str) -> String {
  text.replace([',', '.', '\'', '\u{A0}'], Default::default())
}

/// Convert a timestamp into a date & time string.
pub fn timestamp_to_string(ts: Option<i64>) -> String {
  let Some(ts) = ts else { return String::new() };
  let Some(dt) = DateTime::from_timestamp(ts, 0) else {
    return String::new();
  };
  dt.format("%Y-%m-%d %H:%M:%S").to_string()
}

/// Get the size (thickness) of a scrollbar.
pub fn scroll_bar_size(ui: &Ui) -> f32 {
  let spacing = ui.spacing();
  spacing.scroll.bar_inner_margin + spacing.scroll.bar_width + spacing.scroll.bar_outer_margin
}

/// Get the size (height) of a button.
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
  fn test_floor_search() {
    let values = &[10, 20, 30, 40, 50];
    assert_eq!(Some(3), floor_search(41, values));
    assert_eq!(Some(1), floor_search(27, values));
    assert_eq!(Some(0), floor_search(10, values));
    assert_eq!(None, floor_search(9, values));
    assert_eq!(Some(4), floor_search(180, values));
  }

  #[test]
  fn test_get_countdown_text() {
    assert_eq!("02h 12m 32s", get_countdown_text(HOUR_SECS * 2 + 60 * 12 + 32));
    assert_eq!("07m 44s", get_countdown_text(60 * 7 + 44));
    assert_eq!("05s", get_countdown_text(5));
  }

  #[test]
  fn test_offset() {
    let text = "Is something sub-text?";
    let something = "something";
    let pos = text.find(something).unwrap();
    let substr = &text[pos..pos + something.len()];
    assert_eq!(something, substr);
    assert_eq!(Some(pos), offset(text, substr));
    assert_eq!(None, offset(text, something));
  }

  #[test]
  fn test_to_locale() {
    assert_eq!(Locale::en, to_locale(Some("en-US")));
    assert_eq!(Locale::en_US_POSIX, to_locale(Some("en-US-POSIX")));
    assert_eq!(Locale::ca, to_locale(Some("ca-YT")));
    assert_eq!(Locale::gsw_FR, to_locale(Some("gsw-FR")));
    assert_eq!(Locale::en, to_locale(Some("nope")));
    assert_eq!(Locale::en, to_locale(None));
  }

  #[test]
  fn test_replace_decimal() {
    assert_eq!("123.4", replace_decimal("123.4"));
    assert_eq!("123.4", replace_decimal("123,4"));
    assert_eq!("123.", replace_decimal("123,"));
    assert_eq!(".4", replace_decimal(",4"));
    assert_eq!("123.4", replace_decimal("123\u{66B}4"));
    assert_eq!("123.", replace_decimal("123\u{66B}"));
    assert_eq!(".4", replace_decimal("\u{66B}4"));
  }

  #[test]
  fn test_remove_separators() {
    assert_eq!("123456789", remove_separators("123,456,789"));
    assert_eq!("123456789", remove_separators("123.456.789"));
    assert_eq!("123456789", remove_separators("123'456'789"));
    assert_eq!("123456789", remove_separators("123\u{A0}456\u{A0}789"));
  }

  #[test]
  fn test_find_ignore_case() {
    let text = "Test for 'tschüß' in this text";
    let len = "tschüß".len();
    let result = find_ignore_case(text, "TSCHÜSS");
    assert_eq!(result, Some(10..10 + len));

    let text = "Is 'TSCHÜSS' present?";
    let len = "TSCHÜSS".len();
    let result = find_ignore_case(text, "tschüß");
    assert_eq!(result, Some(4..4 + len));

    let text = "Find 'ghi\u{307}j'";
    let len = "ghi\u{307}j".len();
    let result = find_ignore_case(text, "ghİj");
    assert_eq!(result, Some(6..6 + len));

    let text = "Abc aBc abC";
    let result = find_ignore_case(text, "abc");
    assert_eq!(result, Some(0..3));

    let text = "cbA cBa abC";
    let result = find_ignore_case(text, "abc");
    assert_eq!(result, Some(8..11));
  }

  #[test]
  fn test_timestamp_to_string() {
    let epoch = get_epoch().timestamp();
    assert_eq!("1970-01-01 00:00:00", timestamp_to_string(Some(0)));
    assert_eq!("1997-09-02 00:00:00", timestamp_to_string(Some(epoch)));
  }
}
