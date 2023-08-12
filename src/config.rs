use crate::{
  plant_info::Plant,
  storage::Storage,
  util::{Page, APP_NAME},
};
use eframe::epaint::Pos2;
use std::{
  collections::{BTreeSet, HashMap},
  path::{Path, PathBuf},
};

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

  pub fn get_window_pos(&self) -> Option<Pos2> {
    let text = self.storage.get(WINDOW_POS_KEY)?;
    let pos: Option<(f32, f32)> = ok!(ron::from_str(&text), None);
    pos.map(|pos| pos.into())
  }

  pub fn set_window_pos(&mut self, pos: Option<Pos2>) {
    let pos: Option<(f32, f32)> = pos.map(|pos| pos.into());
    let text = ok!(ron::to_string(&pos));
    self.storage.set(WINDOW_POS_KEY, text);
  }

  pub fn get_page(&self) -> Option<Page> {
    let text = self.storage.get(PAGE_KEY)?;
    Some(ok!(ron::from_str(&text), None))
  }

  pub fn set_page(&mut self, page: Page) {
    let text = ok!(ron::to_string(&page));
    self.storage.set(PAGE_KEY, text);
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
    } else {
      println!("Unable to convert path to string: {path:?}");
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
    } else {
      println!("Unable to convert path to string: {path:?}");
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
  }

  pub fn get_exp_avatar(&self) -> Option<String> {
    self.storage.get(EXP_AVATAR_KEY)
  }

  pub fn set_exp_avatar(&mut self, avatar: String) {
    if avatar.is_empty() {
      return;
    }

    self.storage.set(EXP_AVATAR_KEY, avatar);
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
      return;
    }

    self.storage.set(&key, notes);
  }

  pub fn get_plants(&self) -> Option<Vec<Plant>> {
    let text = self.storage.get(PLANTS_KEY)?;
    Some(ok!(ron::from_str(&text), None))
  }

  pub fn set_plants(&mut self, plants: &Vec<Plant>) {
    // Remove the entry if plants is empty.
    if plants.is_empty() {
      self.storage.remove(PLANTS_KEY);
      return;
    }

    let text = ok!(ron::to_string(plants));
    self.storage.set(PLANTS_KEY, text);
  }

  pub fn get_crop_descriptions(&self) -> Option<BTreeSet<String>> {
    let text = self.storage.get(DESCRIPTIONS_KEY)?;
    Some(ok!(ron::from_str(&text), None))
  }

  pub fn set_crop_descriptions(&mut self, descriptions: &BTreeSet<String>) {
    // Remove the entry if the set is empty.
    if descriptions.is_empty() {
      self.storage.remove(DESCRIPTIONS_KEY);
      return;
    }

    let text = ok!(ron::to_string(descriptions));
    self.storage.set(DESCRIPTIONS_KEY, text);
  }

  pub fn get_avatar_skills(&self, avatar: &str) -> Option<HashMap<u32, (i32, i32)>> {
    if avatar.is_empty() {
      return None;
    }

    let key = format!("{avatar} {AVATAR_SKILLS}");
    let text = self.storage.get(&key)?;
    Some(ok!(ron::from_str(&text), None))
  }

  pub fn set_avatar_skills(&mut self, avatar: &str, skills: &HashMap<u32, (i32, i32)>) {
    if avatar.is_empty() {
      return;
    }

    // Filter out empties.
    let skills: HashMap<u32, (i32, i32)> = skills
      .iter()
      .filter(|(_, levels)| levels.0 > 0 || levels.1 > 0)
      .map(|(id, levels)| (*id, *levels))
      .collect();

    // Remove the entry if skills is empty.
    let key = format!("{avatar} {AVATAR_SKILLS}");
    if skills.is_empty() {
      self.storage.remove(&key);
      return;
    }

    let text = ok!(ron::to_string(&skills));
    self.storage.set(&key, text);
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
}

static WINDOW_POS_KEY: &str = "window_pos";
static LOG_PATH_KEY: &str = "log_path";
static SAVE_PATH_KEY: &str = "save_path";
static STATS_AVATAR_KEY: &str = "stats_avatar";
static EXP_AVATAR_KEY: &str = "experience_avatar";
static AVATAR_SKILLS: &str = "skills";
static PLANTS_KEY: &str = "plants";
static DESCRIPTIONS_KEY: &str = "crop_descriptions";
static NOTES_KEY: &str = "notes";
static PAGE_KEY: &str = "page";
