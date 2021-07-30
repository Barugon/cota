use crate::constants::*;
use crate::thread_pool::*;
use crate::util::*;
use chrono::{NaiveDate, NaiveDateTime, NaiveTime};
use gdnative::api::*;
use gdnative::prelude::*;
use regex::Regex;
use std::{
  cell::RefCell,
  cmp::Reverse,
  collections::HashMap,
  collections::HashSet,
  fs,
  path::{Path, PathBuf},
  str::SplitWhitespace,
  sync::Arc,
};

#[derive(NativeClass)]
#[inherit(Node)]
pub struct Stats {
  config: Config,
  data: RefCell<LogData>,
  view: GodotString,
  avatars: GodotString,
  dates: GodotString,
  notes: GodotString,
  tree: GodotString,
  status: GodotString,
  file_dialog: GodotString,
  filter_dialog: GodotString,
  filter_edit: GodotString,
  notes_dialog: GodotString,
  notes_edit: GodotString,
  search_dialog: GodotString,
  search_edit: GodotString,
  regex_check: GodotString,
  results_dialog: GodotString,
  results_edit: GodotString,
}

enum StatOpts<'a> {
  None,
  Resists,
  Filter(&'a str),
}

#[methods]
impl Stats {
  fn new(_owner: &Node) -> Self {
    let config = Config::new();
    let folder = if let Some(folder) = config.get_log_folder() {
      folder
    } else {
      GodotString::new()
    };
    Stats {
      config,
      data: RefCell::new(LogData::new(&folder)),
      view: GodotString::from("/root/App/VBox/Menu/View"),
      avatars: GodotString::from("Tools/Avatars"),
      dates: GodotString::from("Tools/Dates"),
      notes: GodotString::from("Tools/Notes"),
      tree: GodotString::from("Panel/Tree"),
      status: GodotString::from("Status"),
      file_dialog: GodotString::from("/root/App/FileDialog"),
      filter_dialog: GodotString::from("/root/App/FilterDialog"),
      filter_edit: GodotString::from("/root/App/FilterDialog/VBox/FilterEdit"),
      notes_dialog: GodotString::from("/root/App/NotesDialog"),
      notes_edit: GodotString::from("/root/App/NotesDialog/VBox/NotesEdit"),
      search_dialog: GodotString::from("/root/App/SearchDialog"),
      search_edit: GodotString::from("/root/App/SearchDialog/VBox/SearchEdit"),
      regex_check: GodotString::from("/root/App/SearchDialog/VBox/CheckBox"),
      results_dialog: GodotString::from("/root/App/ResultsDialog"),
      results_edit: GodotString::from("/root/App/ResultsDialog/VBox/ResultsEdit"),
    }
  }

  #[export]
  fn _ready(&self, owner: TRef<Node>) {
    // Connect the view menu and set shortcuts.
    owner.connect_to(&self.view, "id_pressed", "view_menu_select");
    if let Some(button) = owner.get_node_as::<MenuButton>(&self.view) {
      if let Some(popup) = button.get_popup() {
        let popup = popup.to_ref();
        popup.set_shortcut(REFRESH_ID, GlobalConstants::KEY_F5, false);
        popup.set_shortcut(RESISTS_ID, GlobalConstants::KEY_R, true);
        popup.set_shortcut(FILTER_ID, GlobalConstants::KEY_F, true);
        popup.set_shortcut(RESET_ID, GlobalConstants::KEY_ESCAPE, false);
      } else {
        godot_print!("Unable to get popup for {}", self.view);
      }
    }

    // Connect the avatars button.
    owner.connect_to(&self.avatars, "item_selected", "avatar_changed");

    // Connect the dates button.
    owner.connect_to(&self.dates, "item_selected", "date_changed");

    // Connect the notes button.
    owner.connect_to(&self.notes, "pressed", "notes_clicked");

    // Connect the notes dialog.
    owner.connect_to(&self.notes_dialog, "confirmed", "notes_changed");

    // Connect the log folder dialog.
    owner.connect_to(&self.file_dialog, "dir_selected", "log_folder_changed");

    // Connect the filter dialog.
    owner.connect_to(&self.filter_dialog, "confirmed", "filter_changed");
    if let Some(dialog) = owner.get_node_as::<ConfirmationDialog>(&self.filter_dialog) {
      if let Some(edit) = owner.get_node(self.filter_edit.clone()) {
        dialog.register_text_enter(edit);
      }
    }

    // Connect the search dialog.
    owner.connect_to(&self.search_dialog, "confirmed", "search_changed");
    if let Some(dialog) = owner.get_node_as::<ConfirmationDialog>(&self.search_dialog) {
      if let Some(edit) = owner.get_node(self.search_edit.clone()) {
        dialog.register_text_enter(edit);
      }
    }

    // Set some stats tree properties.
    if let Some(tree) = owner.get_node_as::<Tree>(&self.tree) {
      tree.set_column_expand(0, true);
      tree.set_column_min_width(0, 3);
      // tree.set_column_title(0, GodotString::from("Name"));
      // tree.set_column_title(1, GodotString::from("Value"));
      // tree.set_column_titles_visible(true);
    }
    self.populate_avatars(owner);
  }

  #[export]
  fn view_menu_select(&self, owner: TRef<Node>, id: i64) {
    match id {
      REFRESH_ID => self.populate_avatars(owner),
      RESISTS_ID => {
        if let Some(avatar) = self.get_current_avatar(owner) {
          if let Some(ts) = self.get_current_date(owner) {
            let avatar = avatar.to_utf8();
            let avatar = avatar.as_str();
            self.populate_stats(owner, Some(avatar), Some(ts), StatOpts::Resists);
          }
        } else {
          godot_print!("No avatar selected");
        }
      }
      FILTER_ID => {
        if let Some(dialog) = owner.get_node_as::<ConfirmationDialog>(&self.filter_dialog) {
          if let Some(edit) = owner.get_node_as::<LineEdit>(&self.filter_edit) {
            dialog.popup_centered(Vector2::zero());
            edit.grab_focus();
            edit.select_all();
          }
        }
      }
      RESET_ID => {
        if let Some(avatar) = self.get_current_avatar(owner) {
          if let Some(ts) = self.get_current_date(owner) {
            let avatar = avatar.to_utf8();
            let avatar = avatar.as_str();
            self.populate_stats(owner, Some(avatar), Some(ts), StatOpts::None);
          } else {
            godot_print!("No date/time selected");
          }
        } else {
          godot_print!("No avatar selected");
        }
      }
      _ => {}
    }
  }

  #[export]
  fn avatar_changed(&self, owner: TRef<Node>, item: i64) {
    if let Some(button) = owner.get_node_as::<OptionButton>(&self.avatars) {
      let avatar = button.get_item_text(item);
      self.config.set_avatar(Some(&avatar));

      if !avatar.is_empty() {
        let avatar = avatar.to_utf8();
        let avatar = avatar.as_str();
        self.populate_dates(owner, Some(avatar));
        return;
      }
    }
    self.populate_dates(owner, None);
  }

  #[export]
  fn date_changed(&self, owner: TRef<Node>, item: i64) {
    if let Some(avatar) = self.get_current_avatar(owner) {
      if let Some(button) = owner.get_node_as::<OptionButton>(&self.dates) {
        let ts = button.get_item_id(item);
        if ts != 0 {
          let avatar = avatar.to_utf8();
          let avatar = avatar.as_str();
          self.populate_stats(owner, Some(avatar), Some(ts), StatOpts::None);
          return;
        }
      }
    }
    self.populate_stats(owner, None, None, StatOpts::None);
  }

  #[export]
  fn log_folder_changed(&self, owner: TRef<Node>, folder: GodotString) {
    *self.data.borrow_mut() = LogData::new(&folder);
    self.config.set_log_folder(Some(&folder));
    self.populate_avatars(owner);
  }

  #[export]
  fn filter_changed(&self, owner: TRef<Node>) {
    if let Some(edit) = owner.get_node_as::<LineEdit>(&self.filter_edit) {
      let text = edit.text();
      if !text.is_empty() {
        let text = text.to_utf8();
        let text = text.as_str();
        if let Some(avatar) = self.get_current_avatar(owner) {
          if let Some(ts) = self.get_current_date(owner) {
            let avatar = avatar.to_utf8();
            let avatar = avatar.as_str();
            self.populate_stats(owner, Some(avatar), Some(ts), StatOpts::Filter(text));
          }
        }
      }
    }
  }

  #[export]
  fn notes_clicked(&self, owner: TRef<Node>) {
    if let Some(dialog) = owner.get_node_as::<ConfirmationDialog>(&self.notes_dialog) {
      if let Some(edit) = owner.get_node_as::<TextEdit>(&self.notes_edit) {
        if let Some(avatar) = self.get_current_avatar(owner) {
          let title = GodotString::from(format!("Notes for {}", avatar.to_utf8().as_str()));
          let text = if let Some(text) = self.config.get_notes(&avatar) {
            text
          } else {
            GodotString::new()
          };
          edit.set_text(text);
          dialog.set_title(title);
          dialog.popup_centered(Vector2::zero());
          edit.grab_focus();
        }
      }
    }
  }

  #[export]
  fn notes_changed(&self, owner: TRef<Node>) {
    if let Some(edit) = owner.get_node_as::<TextEdit>(&self.notes_edit) {
      let text = edit.text();
      if let Some(avatar) = self.get_current_avatar(owner) {
        self.config.set_notes(&avatar, Some(&text));
      }
    }
  }

  #[export]
  fn search(&self, owner: TRef<Node>) {
    if let Some(dialog) = owner.get_node_as::<ConfirmationDialog>(&self.search_dialog) {
      if let Some(edit) = owner.get_node_as::<LineEdit>(&self.search_edit) {
        dialog.popup_centered(Vector2::zero());
        edit.grab_focus();
        edit.select_all();
      }
    }
  }

  #[export]
  fn search_changed(&self, owner: TRef<Node>) {
    if let Some(edit) = owner.get_node_as::<LineEdit>(&self.search_edit) {
      let text = edit.text();
      if !text.is_empty() {
        if let Some(regex) = owner.get_node_as::<CheckBox>(&self.regex_check) {
          // Get the search term.
          let search = if regex.is_pressed() {
            Search::R(Box::new(ok!(Regex::new(text.to_utf8().as_str()))))
          } else {
            Search::S(String::from(text.to_utf8().as_str()))
          };
          // Find matching log entries.
          let text = if let Some(avatar) = self.get_current_avatar(owner) {
            self.find_log_entries(avatar.to_utf8().as_str(), search)
          } else {
            String::new()
          };
          // Display the log entries in a dialog.
          if let Some(dialog) = owner.get_node_as::<WindowDialog>(&self.results_dialog) {
            if let Some(edit) = owner.get_node_as::<TextEdit>(&self.results_edit) {
              edit.set_text(text);
              edit.clear_undo_history();
              dialog.popup_centered(Vector2::zero());
            }
          }
        }
      }
    }
  }

  fn find_log_entries(&self, avatar: &str, search: Search) -> String {
    self.data.borrow().find_log_entries(avatar, search)
  }

  fn get_avatars(&self) -> Vec<String> {
    self.data.borrow().get_avatars()
  }

  fn populate_avatars(&self, owner: TRef<Node>) {
    if let Some(button) = owner.get_node_as::<OptionButton>(&self.avatars) {
      self.enable_notes(owner, false);
      button.clear();

      let names = self.get_avatars();
      for (idx, name) in names.iter().enumerate() {
        button.add_item(GodotString::from(name), idx as i64 + 1);
      }

      if button.get_item_count() > 0 {
        if let Some(avatar) = self.config.get_avatar() {
          button.select_item(&avatar);
        }

        let avatar = button.get_item_text(button.selected());
        if !avatar.is_empty() {
          self.enable_notes(owner, true);
          self.populate_dates(owner, Some(avatar.to_utf8().as_str()));
          return;
        }
      }
    }
    self.populate_dates(owner, None);
  }

  fn get_current_avatar(&self, owner: TRef<Node>) -> Option<GodotString> {
    if let Some(button) = owner.get_node_as::<OptionButton>(&self.avatars) {
      let id = button.get_selected_id();
      if id != 0 {
        let avatar = button.get_item_text(button.get_item_index(id));
        if !avatar.is_empty() {
          return Some(avatar);
        }
      }
    }
    None
  }

  fn get_stats_timestamps(&self, avatar: &str) -> Vec<i64> {
    self.data.borrow().get_stats_timestamps(avatar)
  }

  fn populate_dates(&self, owner: TRef<Node>, avatar: Option<&str>) {
    if let Some(button) = owner.get_node_as::<OptionButton>(&self.dates) {
      button.clear();
      if let Some(avatar) = avatar {
        let timestamps = self.get_stats_timestamps(avatar);
        if !timestamps.is_empty() {
          for ts in timestamps {
            let date = timestamp_to_view_date(ts);
            button.add_item(GodotString::from(date), ts);
          }

          let ts = button.get_item_id(0);
          if ts != 0 {
            self.populate_stats(owner, Some(avatar), Some(ts), StatOpts::None);
            return;
          }
        }
      }
    }
    self.populate_stats(owner, avatar, None, StatOpts::None);
  }

  fn get_current_date(&self, owner: TRef<Node>) -> Option<i64> {
    if let Some(button) = owner.get_node_as::<OptionButton>(&self.dates) {
      let ts = button.get_selected_id();
      if ts != 0 {
        return Some(ts);
      }
    }
    None
  }

  fn enable_notes(&self, owner: TRef<Node>, enable: bool) {
    if let Some(button) = owner.get_node_as::<Button>(&self.notes) {
      button.set_disabled(!enable);
      button.set_focus_mode(if enable {
        Control::FOCUS_ALL
      } else {
        Control::FOCUS_NONE
      });
    }
  }

  fn get_stats(&self, avatar: &str, ts: i64) -> Option<StatsData> {
    self.data.borrow().get_stats(avatar, ts)
  }

  fn populate_stats(
    &self,
    owner: TRef<Node>,
    avatar: Option<&str>,
    ts: Option<i64>,
    opts: StatOpts,
  ) {
    self.set_status_message(owner, None);

    let tree = some!(owner.get_node_as::<Tree>(&self.tree));
    tree.clear();
    tree.set_focus_mode(Control::FOCUS_NONE as i64);

    let avatar = some!(avatar);
    if let Some(ts) = ts {
      if let Some(stats) = self.get_stats(avatar, ts) {
        if let Some(parent) = tree.create_item(Object::null(), -1) {
          let locale = get_locale();
          let mut bg_color = Cycle::new(vec![
            Color::rgb(0.18, 0.18, 0.18),
            Color::rgb(0.16, 0.16, 0.16),
          ]);

          match opts {
            StatOpts::Resists => {
              #[derive(Hash, Eq, PartialEq, Copy, Clone)]
              enum Resist {
                Air,
                Chaos,
                Death,
                Earth,
                Fire,
                Life,
                Moon,
                Sun,
                Water,
                Magic,
              }
              const RESIST_STATS: [(&str, (Resist, f64)); 19] = [
                ("AirAttunement", (Resist::Air, 0.5)),
                ("AirResistance", (Resist::Air, 1.0)),
                ("ChaosAttunement", (Resist::Chaos, 0.5)),
                ("ChaosResistance", (Resist::Chaos, 1.0)),
                ("DeathAttunement", (Resist::Death, 0.5)),
                ("DeathResistance", (Resist::Death, 1.0)),
                ("EarthAttunement", (Resist::Earth, 0.5)),
                ("EarthResistance", (Resist::Earth, 1.0)),
                ("FireAttunement", (Resist::Fire, 0.5)),
                ("FireResistance", (Resist::Fire, 1.0)),
                ("LifeAttunement", (Resist::Life, 0.5)),
                ("LifeResistance", (Resist::Life, 1.0)),
                ("MoonAttunement", (Resist::Moon, 0.5)),
                ("MoonResistance", (Resist::Moon, 1.0)),
                ("SunAttunement", (Resist::Sun, 0.5)),
                ("SunResistance", (Resist::Sun, 1.0)),
                ("WaterAttunement", (Resist::Water, 0.5)),
                ("WaterResistance", (Resist::Water, 1.0)),
                ("MagicResistance", (Resist::Magic, 1.0)),
              ];
              const RESIST_NAMES: [&str; 9] = [
                "Air", "Chaos", "Death", "Earth", "Fire", "Life", "Moon", "Sun", "Water",
              ];
              const RESIST_KEYS: [Resist; 9] = [
                Resist::Air,
                Resist::Chaos,
                Resist::Death,
                Resist::Earth,
                Resist::Fire,
                Resist::Life,
                Resist::Moon,
                Resist::Sun,
                Resist::Water,
              ];
              let resist_stats: HashMap<&str, (Resist, f64)> =
                RESIST_STATS.iter().cloned().collect();
              let mut resist_values: HashMap<Resist, f64> = HashMap::new();

              // Collect and sum the resistances.
              for (name, value) in stats.iter() {
                if let Some((key, mul)) = resist_stats.get(name) {
                  // Stats possibly use ',' as the decimal separator depending on locale.
                  if let Ok(val) = value.replacen(',', ".", 1).parse::<f64>() {
                    if let Some(resist) = resist_values.get_mut(key) {
                      *resist += val * mul;
                    } else {
                      resist_values.insert(*key, val * mul);
                    }
                  }
                }
              }

              // Add-in magic resistance.
              if let Some(magic) = resist_values.remove(&Resist::Magic) {
                for (key, resist) in &mut resist_values {
                  if *key != Resist::Chaos {
                    *resist += magic;
                  }
                }
              }

              // Format the output.
              for (pos, key) in RESIST_KEYS.iter().enumerate() {
                if let Some(value) = resist_values.get(key) {
                  if let Some(item) = tree.create_item(parent, -1) {
                    let item = item.to_ref();
                    let name = RESIST_NAMES[pos];
                    let value = value.to_display_string(locale);
                    let bg = *bg_color.get();

                    item.set_selectable(0, false);
                    item.set_selectable(1, false);
                    item.set_custom_bg_color(0, bg, false);
                    item.set_custom_bg_color(1, bg, false);
                    item.set_custom_color(0, Color::rgb(0.7, 0.6, 0.4));
                    item.set_text(0, GodotString::from(name));
                    item.set_text(1, GodotString::from(value));
                  }
                }
              }

              let text = format!(
                "Showing effective resists from {}",
                timestamp_to_view_date(ts)
              );
              self.set_status_message(owner, Some(&text));
              tree.set_focus_mode(Control::FOCUS_ALL as i64);
              return;
            }
            _ => {
              for (name, value) in stats.iter() {
                if let StatOpts::Filter(filter) = opts {
                  // Check if the name contains the filter string.
                  if !ascii_contains_ignore_case(name.as_bytes(), filter.as_bytes()) {
                    continue;
                  }
                }

                // Stats possibly use ',' as the decimal separator depending on locale.
                if let Ok(value) = value.replacen(',', ".", 1).parse::<f64>() {
                  if let Some(item) = tree.create_item(parent, -1) {
                    let item = item.to_ref();
                    let value = &value.to_display_string(locale);
                    let bg = *bg_color.get();

                    item.set_selectable(0, false);
                    item.set_selectable(1, false);
                    item.set_custom_bg_color(0, bg, false);
                    item.set_custom_bg_color(1, bg, false);
                    item.set_custom_color(0, Color::rgb(0.4, 0.6, 0.7));
                    item.set_text(0, GodotString::from(name));
                    item.set_text(1, GodotString::from(value));
                  }
                }
              }

              let date = timestamp_to_view_date(ts);
              let text = match opts {
                StatOpts::Filter(_) => format!("Showing filtered stats from {}", date),
                _ => format!("Showing stats from {}", date),
              };
              self.set_status_message(owner, Some(&text));
              tree.set_focus_mode(Control::FOCUS_ALL as i64);
              return;
            }
          }
        }
      }
    }

    let text = format!("No stats found for {}", avatar);
    self.set_status_message(owner, Some(&text));
  }

  fn set_status_message(&self, owner: TRef<Node>, text: Option<&str>) {
    if let Some(label) = owner.get_node_as::<Label>(&self.status) {
      match text {
        Some(text) => label.set_text(GodotString::from(text)),
        None => label.set_text(GodotString::new()),
      }
    }
  }
}

/// Convert a timestamp into a date & time string.
fn timestamp_to_view_date(ts: i64) -> String {
  NaiveDateTime::from_timestamp(ts, 0)
    .format("%Y-%m-%d @ %H:%M:%S")
    .to_string()
}

// Convert a SotA log date & time into a timestamp. Since the dates are localized, we don't know
// if day or month come first, so we use the date from the filename, which is always YYYY-MM-DD.
fn log_date_to_timestamp(text: &str, date: &NaiveDate) -> Option<i64> {
  let mut iter = text.split_whitespace();
  let _date = some!(iter.next(), None);
  let time = some!(iter.next(), None);
  let ap = iter.next();

  if iter.next().is_some() {
    return None;
  }

  let mut iter = time.split(':');
  let parts = [
    some!(iter.next(), None),
    some!(iter.next(), None),
    some!(iter.next(), None),
  ];

  if iter.next().is_some() {
    return None;
  }

  // Parse the hour and adjust for PM.
  let hour = {
    let mut hour = ok!(parts[0].parse(), None);
    if let Some(ap) = ap {
      let bytes = ap.as_bytes();
      if !bytes.is_empty() {
        let ch = bytes[0] as char;
        if ch == 'P' || ch == 'p' {
          hour += 12;
          if hour == 24 {
            hour = 0;
          }
        }
      }
    }
    hour
  };

  let minute = ok!(parts[1].parse(), None);
  let second = ok!(parts[2].parse(), None);

  Some(NaiveDateTime::new(*date, NaiveTime::from_hms(hour, minute, second)).timestamp())
}

// Convert a timestamp into a log filename date string.
fn timestamp_to_file_date(ts: i64) -> String {
  NaiveDateTime::from_timestamp(ts, 0)
    .format("%Y-%m-%d")
    .to_string()
}

fn get_log_file_date(path: &Path) -> Option<NaiveDate> {
  let filename = some!(path.file_stem(), None);
  let filename = some!(filename.to_str(), None);
  let pos = some!(filename.rfind('_'), None);
  let text = &filename[pos + 1..];

  if let Ok(date) = NaiveDate::parse_from_str(text, "%Y-%m-%d") {
    return Some(date);
  }

  None
}

fn get_stats_timestamp(line: &str, date: &NaiveDate) -> Option<i64> {
  if line.starts_with('[') {
    if let Some(pos) = line.find(']') {
      if line[pos + 1..].contains(STATS_KEY) {
        return log_date_to_timestamp(&line[1..pos], date);
      }
    }
  }

  None
}

fn get_stats_text<'a>(line: &'a str, ts: i64, date: &NaiveDate) -> Option<&'a str> {
  if let Some(lts) = get_stats_timestamp(line, date) {
    if lts == ts {
      if let Some(pos) = line.rfind(']') {
        return Some(&line[pos + 1..]);
      }
    }
  }

  None
}

const FILENAME_START: &str = "SotAChatLog_";
const STATS_KEY: &str = " AdventurerLevel: ";

struct StatsIter<'a> {
  iter: SplitWhitespace<'a>,
}

impl<'a> StatsIter<'a> {
  fn new(text: &str) -> StatsIter {
    StatsIter {
      iter: text.split_whitespace(),
    }
  }
}

impl<'a> Iterator for StatsIter<'a> {
  type Item = (&'a str, &'a str);

  fn next(&mut self) -> Option<Self::Item> {
    for name in &mut self.iter {
      if let Some(name) = name.strip_suffix(':') {
        if let Some(value) = self.iter.next() {
          return Some((name, value));
        }
        break;
      }
    }

    None
  }
}

struct StatsData {
  text: String,
}

impl StatsData {
  fn new(text: String) -> StatsData {
    StatsData { text }
  }

  fn iter(&self) -> StatsIter<'_> {
    StatsIter::new(&self.text)
  }
}

enum Search {
  S(String),
  R(Box<Regex>),
}

/// Object that reads from SotA chat logs.
struct LogData {
  folder: PathBuf,
  pool: RefCell<ThreadPool>,
}

impl LogData {
  fn new(folder: &GodotString) -> LogData {
    let cpus = std::cmp::max(num_cpus::get(), 2);
    LogData {
      folder: PathBuf::from(folder.to_utf8().as_str()),
      pool: RefCell::new(ThreadPool::new(cpus)),
    }
  }

  /// Get a vector of avatar names.
  fn get_avatars(&self) -> Vec<String> {
    let filenames = self.get_log_filenames(None, None);
    let mut name_set = HashSet::<&str>::new();

    for filename in &filenames {
      let filename = &filename[FILENAME_START.len()..];
      if let Some(pos) = filename.rfind('_') {
        name_set.insert(&filename[..pos]);
      }
    }

    let mut avatars = Vec::with_capacity(name_set.len());
    for name in name_set {
      avatars.push(String::from(name));
    }

    avatars.sort_unstable();
    avatars
  }

  /// Get a vector of timestamps where `/stats` was used for the specified avatar.
  fn get_stats_timestamps(&self, avatar: &str) -> Vec<i64> {
    let tasks = {
      let filenames = self.get_log_filenames(Some(avatar), None);
      let mut tasks = Vec::new();
      let mut pool = self.pool.borrow_mut();
      for filename in filenames {
        let path = self.folder.join(filename.as_str());
        if let Some(date) = get_log_file_date(&path) {
          // Each task will read and scan one log file.
          let task = pool.exec(move |cancel| {
            let mut timestamps = Vec::new();
            if let Ok(text) = fs::read_to_string(&path) {
              for line in text.lines() {
                if cancel() {
                  break;
                }
                if let Some(ts) = get_stats_timestamp(line, &date) {
                  timestamps.push(ts);
                }
              }
            }
            Some(timestamps)
          });
          tasks.push(task);
        }
      }
      tasks
    };

    let mut timestamps = Vec::new();
    for mut task in tasks {
      // Concatenate the results.
      if let Some(mut result) = task.get() {
        timestamps.append(&mut result);
      }
    }

    // Sort the timestamps so that the most recent is first.
    timestamps.sort_unstable_by_key(|&key| Reverse(key));
    timestamps
  }

  /// Get the stats for the specified avatar and timestamp.
  fn get_stats(&self, avatar: &str, ts: i64) -> Option<StatsData> {
    let filenames = self.get_log_filenames(Some(avatar), Some(ts));

    // There will actually only be one file with the specific avatar name and date.
    for filename in filenames {
      let path = self.folder.join(filename.as_str());
      if let Some(date) = get_log_file_date(&path) {
        if let Ok(text) = fs::read_to_string(path) {
          for line in text.lines() {
            if let Some(mut stats) = get_stats_text(line, ts, &date) {
              if stats.len() < 1000 {
                // A Lua script has probably inserted newlines.
                let pos = stats.as_ptr() as usize - text.as_ptr() as usize;
                let mut end = pos + stats.len();

                // Collect subsequent lines that don't have a timestamp.
                for line in text[end..].lines() {
                  end = line.as_ptr() as usize - text.as_ptr() as usize;
                  if line.starts_with('[') {
                    break;
                  }
                }
                stats = &text[pos..end];
              }
              return Some(StatsData::new(String::from(stats)));
            }
          }
        }
      }
    }

    None
  }

  /// Find log entries matching the provided search term.
  fn find_log_entries(&self, avatar: &str, search: Search) -> String {
    // Godot will choke if the text is too big, so we limit
    // it to the most recent megabyte of text.
    const LIMIT: usize = 1048576;

    let tasks = {
      let search = Arc::new(search);
      let mut filenames = self.get_log_filenames(Some(avatar), None);
      let mut tasks = Vec::with_capacity(filenames.len());
      let mut pool = self.pool.borrow_mut();
      // Work on files from newest to oldest.
      filenames.sort_unstable_by(|a, b| b.cmp(a));
      for filename in filenames {
        let path = self.folder.join(filename);
        let search = Arc::clone(&search);
        let task = pool.exec(move |cancel| {
          let mut lines = Vec::new();
          let mut size: usize = 0;
          if let Ok(text) = fs::read_to_string(&path) {
            for line in text.lines() {
              if cancel() {
                break;
              } else {
                match search.as_ref() {
                  Search::S(search) => {
                    if line.contains(search) {
                      size += line.len();
                      lines.push(line);
                    }
                  }
                  Search::R(search) => {
                    if search.is_match(line) {
                      size += line.len();
                      lines.push(line);
                    }
                  }
                }
              }
            }
            let mut result = String::with_capacity(size + lines.len());
            size = 0;
            // Concatenate the lines in reverse order (newest to oldest).
            for line in lines.iter().rev() {
              result.push_str(line);
              result.push('\n');
              // If the ultimate size limit is reached then we're done.
              size += line.len() + 1;
              if size >= LIMIT {
                break;
              }
            }
            return Some(result);
          }
          None
        });
        tasks.push(task);
      }
      tasks
    };

    // Collect the text from each task.
    let mut results = Vec::with_capacity(tasks.len());
    let mut size: usize = 0;
    for mut task in tasks {
      if size >= LIMIT {
        // Size limit reached. Cancel the remaining tasks.
        task.cancel();
      } else if let Some(result) = task.get() {
        size += result.len();
        results.push(result);
      }
    }

    // Collect the lines of text up to the size limit.
    size = 0;
    let mut lines = Vec::new();
    for result in &results {
      for line in result.lines() {
        if !line.is_empty() {
          size += line.len();
          lines.push(line);
          if size >= LIMIT {
            break;
          }
        }
      }
    }

    // Concatenate the lines of text in reverse order.
    let mut text = String::with_capacity(size + lines.len());
    for line in lines.iter().rev() {
      text.push_str(line);
      text.push('\n');
    }
    text
  }

  fn get_log_filenames(&self, avatar: Option<&str>, ts: Option<i64>) -> Vec<String> {
    let mut filenames = Vec::new();
    let entries = ok!(self.folder.read_dir(), filenames);

    // The name text is either a specific avatar or a regex wildcard.
    let name = if let Some(avatar) = avatar {
      avatar
    } else {
      ".+"
    };

    // The date text is either a specific date or regex to match the date.
    let date = if let Some(ts) = ts {
      format!("_{}", timestamp_to_file_date(ts))
    } else {
      String::from(r"_\d{4}-\d{2}-\d{2}")
    };

    let regex = ok!(
      Regex::new(&format!("^{}{}{}.txt$", FILENAME_START, name, date)),
      filenames
    );

    for entry in entries.flatten() {
      if let Ok(filename) = entry.file_name().into_string() {
        if regex.is_match(&filename) {
          filenames.push(filename);
        }
      }
    }

    filenames
  }
}
