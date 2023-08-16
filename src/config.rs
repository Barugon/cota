use crate::{
  plant_info::CropTimer,
  storage::Storage,
  util::{Page, APP_NAME},
};
use eframe::epaint::Pos2;
use std::{
  collections::{BTreeMap, BTreeSet, HashMap},
  path::{Path, PathBuf},
};

static WINDOW_POS_KEY: &str = "window_pos";
static LOG_PATH_KEY: &str = "log_path";
static SAVE_PATH_KEY: &str = "save_path";
static STATS_AVATAR_KEY: &str = "stats_avatar";
static EXP_AVATAR_KEY: &str = "experience_avatar";
static AVATAR_SKILLS: &str = "skills";
static CROP_TIMERS_KEY: &str = "plants";
static CROP_DESCRIPTIONS_KEY: &str = "crop_descriptions";
static NOTES_KEY: &str = "notes";
static PAGE_KEY: &str = "page";

/// Companion of the Avatar configuration storage.
#[derive(Clone)]
pub struct Config {
  storage: Storage,
}

impl Config {
  pub fn new() -> Option<Self> {
    let path = Self::path()?;
    let storage = Storage::new(path)?;
    Some(Self { storage })
  }

  fn path() -> Option<PathBuf> {
    dirs::config_dir().map(|path| path.join(APP_NAME).with_extension("ron"))
  }

  fn get_sota_config_path() -> Option<PathBuf> {
    let path = dirs::config_dir()?;
    Some(path.join("Portalarium").join("Shroud of the Avatar"))
  }

  fn get_default_log_path() -> Option<PathBuf> {
    if let Some(path) = Self::get_sota_config_path() {
      let path = path.join("ChatLogs");
      if path.is_dir() {
        return Some(path);
      }
    }
    dirs::home_dir()
  }

  fn get_default_save_path() -> Option<PathBuf> {
    if let Some(path) = Self::get_sota_config_path() {
      let path = path.join("SavedGames");
      if path.is_dir() {
        return Some(path);
      }
    }
    dirs::home_dir()
  }

  pub fn get_window_pos(&self) -> Option<Pos2> {
    let pos: (f32, f32) = self.storage.get_as(WINDOW_POS_KEY)?;
    Some(pos.into())
  }

  pub fn set_window_pos(&mut self, pos: Option<Pos2>) {
    if let Some(pos) = pos {
      let pos: (f32, f32) = pos.into();
      self.storage.set_as(WINDOW_POS_KEY, &pos);
    } else {
      self.storage.remove(WINDOW_POS_KEY);
    }
  }

  pub fn get_page(&self) -> Option<Page> {
    self.storage.get_as(PAGE_KEY)
  }

  pub fn set_page(&mut self, page: Page) {
    self.storage.set_as(PAGE_KEY, &page);
    self.storage.persist();
  }

  pub fn get_log_path(&self) -> Option<PathBuf> {
    if let Some(path) = self.storage.get(LOG_PATH_KEY) {
      return Some(PathBuf::from(path));
    }

    Self::get_default_log_path()
  }

  pub fn set_log_path(&mut self, path: &Path) {
    if let Some(path) = path.to_str() {
      self.storage.set(LOG_PATH_KEY, path.to_owned());
      self.storage.persist();
    } else {
      println!("Invalid unicode in path: {path:?}");
    }
  }

  pub fn get_save_path(&self) -> Option<PathBuf> {
    if let Some(path) = self.storage.get(SAVE_PATH_KEY) {
      return Some(PathBuf::from(path));
    }

    Self::get_default_save_path()
  }

  pub fn set_save_path(&mut self, path: &Path) {
    if let Some(path) = path.to_str() {
      self.storage.set(SAVE_PATH_KEY, path.to_owned());
      self.storage.persist();
    } else {
      println!("Invalid unicode in path: {path:?}");
    }
  }

  pub fn get_stats_avatar(&self) -> Option<String> {
    self.storage.get(STATS_AVATAR_KEY)
  }

  pub fn set_stats_avatar(&mut self, avatar: String) {
    if avatar.is_empty() {
      return;
    }

    self.storage.set(STATS_AVATAR_KEY, avatar);
    self.storage.persist();
  }

  pub fn get_exp_avatar(&self) -> Option<String> {
    self.storage.get(EXP_AVATAR_KEY)
  }

  pub fn set_exp_avatar(&mut self, avatar: String) {
    if avatar.is_empty() {
      return;
    }

    self.storage.set(EXP_AVATAR_KEY, avatar);
    self.storage.persist();
  }

  pub fn get_notes(&self, avatar: &str) -> Option<String> {
    if avatar.is_empty() {
      return None;
    }

    let key = format!("{avatar} {NOTES_KEY}");
    self.storage.get(&key)
  }

  pub fn set_notes(&mut self, avatar: &str, notes: String) {
    if avatar.is_empty() {
      return;
    }

    // Remove the entry if notes is empty.
    let key = format!("{avatar} {NOTES_KEY}");
    if notes.is_empty() {
      self.storage.remove(&key);
    } else {
      self.storage.set(&key, notes);
    }

    self.storage.persist();
  }

  pub fn get_crop_timers(&self) -> Option<Vec<CropTimer>> {
    self.storage.get_as(CROP_TIMERS_KEY)
  }

  pub fn set_crop_timers(&mut self, timers: &Vec<CropTimer>) {
    // Remove the entry if timers is empty.
    if timers.is_empty() {
      self.storage.remove(CROP_TIMERS_KEY);
    } else {
      self.storage.set_as(CROP_TIMERS_KEY, timers);
    }

    self.storage.persist();
  }

  pub fn get_crop_descriptions(&self) -> Option<BTreeSet<String>> {
    self.storage.get_as(CROP_DESCRIPTIONS_KEY)
  }

  pub fn set_crop_descriptions(&mut self, descriptions: &BTreeSet<String>) {
    // Remove the entry if descriptions is empty.
    if descriptions.is_empty() {
      self.storage.remove(CROP_DESCRIPTIONS_KEY);
    } else {
      self.storage.set_as(CROP_DESCRIPTIONS_KEY, descriptions);
    }

    self.storage.persist();
  }

  pub fn get_avatar_skills(&self, avatar: &str) -> Option<HashMap<u32, (i32, i32)>> {
    if avatar.is_empty() {
      return None;
    }

    let key = format!("{avatar} {AVATAR_SKILLS}");
    self.storage.get_as(&key)
  }

  pub fn set_avatar_skills(&mut self, avatar: &str, skills: &HashMap<u32, (i32, i32)>) {
    if avatar.is_empty() {
      return;
    }

    // Filter out empties. Use BTreeMap so that the entries are sorted.
    let skills: BTreeMap<u32, (i32, i32)> = skills
      .iter()
      .filter(|(_, levels)| levels.0 > 0 || levels.1 > 0)
      .map(|(id, levels)| (*id, *levels))
      .collect();

    // Remove the entry if skills is empty.
    let key = format!("{avatar} {AVATAR_SKILLS}");
    if skills.is_empty() {
      self.storage.remove(&key);
    } else {
      self.storage.set_as(&key, &skills);
    }

    self.storage.persist();
  }
}
