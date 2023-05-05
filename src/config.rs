use crate::plant_info::Plant;
use eframe::Storage;
use std::{
  collections::HashMap,
  path::{Path, PathBuf},
};

const LOG_PATH_KEY: &str = "log_path";
const SAVE_PATH_KEY: &str = "save_path";
const STATS_AVATAR_KEY: &str = "stats_avatar";
const EXP_AVATAR_KEY: &str = "experience_avatar";
const AVATAR_SKILLS: &str = "skills";
const PLANTS_KEY: &str = "plants";
const NOTES_KEY: &str = "notes";

pub fn get_log_path(storage: &dyn Storage) -> Option<PathBuf> {
  if let Some(folder) = storage.get_string(LOG_PATH_KEY) {
    Some(PathBuf::from(folder))
  } else {
    get_default_log_path()
  }
}

pub fn set_log_path(storage: &mut dyn Storage, folder: &Path) {
  if let Some(folder) = folder.to_str() {
    storage.set_string(LOG_PATH_KEY, folder.to_owned());
  } else {
    println!("Unable to convert path to string: {folder:?}");
  }
}

pub fn get_save_path(storage: &dyn Storage) -> Option<PathBuf> {
  if let Some(folder) = storage.get_string(SAVE_PATH_KEY) {
    Some(PathBuf::from(folder))
  } else {
    get_default_save_path()
  }
}

pub fn set_save_path(storage: &mut dyn Storage, folder: &Path) {
  if let Some(folder) = folder.to_str() {
    storage.set_string(SAVE_PATH_KEY, folder.to_owned());
  } else {
    println!("Unable to convert path to string: {folder:?}");
  }
}

pub fn get_stats_avatar(storage: &dyn Storage) -> Option<String> {
  storage.get_string(STATS_AVATAR_KEY)
}

pub fn set_stats_avatar(storage: &mut dyn Storage, avatar: String) {
  storage.set_string(STATS_AVATAR_KEY, avatar);
}

pub fn get_exp_avatar(storage: &dyn Storage) -> Option<String> {
  storage.get_string(EXP_AVATAR_KEY)
}

pub fn set_exp_avatar(storage: &mut dyn Storage, avatar: String) {
  storage.set_string(EXP_AVATAR_KEY, avatar);
}

pub fn get_notes(storage: &dyn Storage, avatar: &str) -> Option<String> {
  if avatar.is_empty() {
    return None;
  }

  let key = format!("{avatar} {NOTES_KEY}");
  storage.get_string(&key)
}

pub fn set_notes(storage: &mut dyn Storage, avatar: &str, notes: String) {
  if !avatar.is_empty() {
    let key = format!("{avatar} {NOTES_KEY}");
    storage.set_string(&key, notes);
  }
}

pub fn get_plants(storage: &dyn Storage) -> Option<Vec<Plant>> {
  let text = storage.get_string(PLANTS_KEY)?;
  Some(ok!(ron::from_str(&text), None))
}

pub fn set_plants(storage: &mut dyn Storage, plants: &Vec<Plant>) {
  let text = ok!(ron::to_string(plants));
  storage.set_string(PLANTS_KEY, text);
}

pub fn get_avatar_skills(
  storage: &mut dyn Storage,
  avatar: &str,
) -> Option<HashMap<u32, (i32, i32)>> {
  if avatar.is_empty() {
    return None;
  }

  let key = format!("{avatar} {AVATAR_SKILLS}");
  let text = storage.get_string(&key)?;
  Some(ok!(ron::from_str(&text), None))
}

pub fn set_avatar_skills(
  storage: &mut dyn Storage,
  avatar: &str,
  skills: &HashMap<u32, (i32, i32)>,
) {
  if avatar.is_empty() {
    return;
  }

  // Filter out empties.
  let skills: HashMap<u32, (i32, i32)> = skills
    .iter()
    .filter(|(_, levels)| levels.0 > 0 || levels.1 > 0)
    .map(|(id, levels)| (*id, *levels))
    .collect();

  let text = ok!(ron::to_string(&skills));
  let key = format!("{avatar} {AVATAR_SKILLS}");
  storage.set_string(&key, text);
}

fn get_sota_config_path() -> Option<PathBuf> {
  let path = dirs::config_dir()?;

  // Concatenate using join for correct separators.
  Some(path.join("Portalarium").join("Shroud of the Avatar"))
}

fn get_default_log_path() -> Option<PathBuf> {
  if let Some(path) = get_sota_config_path() {
    let path = path.join("ChatLogs");
    if path.is_dir() {
      return Some(path);
    }
  }
  dirs::home_dir()
}

fn get_default_save_path() -> Option<PathBuf> {
  if let Some(path) = get_sota_config_path() {
    let path = path.join("SavedGames");
    if path.is_dir() {
      return Some(path);
    }
  }
  dirs::home_dir()
}
