use crate::plant_info::Plant;
use eframe::Storage;
use std::path::{Path, PathBuf};

const LOG_PATH_KEY: &str = "log_path";
const SAVE_PATH_KEY: &str = "save_path";
const PLANTS_KEY: &str = "plants";
const AVATAR_KEY: &str = "avatar";
const NOTES_KEY: &str = "notes";

pub fn get_sota_config_path() -> Option<PathBuf> {
  let path = dirs::config_dir()?;

  // Concatenate using join for correct separators.
  Some(path.join("Portalarium").join("Shroud of the Avatar"))
}

pub fn get_log_path(storage: &dyn Storage) -> Option<PathBuf> {
  if let Some(folder) = get_value(storage, LOG_PATH_KEY) {
    Some(PathBuf::from(folder))
  } else {
    get_default_log_path()
  }
}

pub fn get_save_path(storage: &dyn Storage) -> Option<PathBuf> {
  if let Some(folder) = get_value(storage, SAVE_PATH_KEY) {
    Some(PathBuf::from(folder))
  } else {
    get_default_save_path()
  }
}

pub fn set_log_path(storage: &mut dyn Storage, folder: &Path) {
  if let Some(folder) = folder.to_str() {
    set_value(storage, LOG_PATH_KEY, folder.to_owned());
  } else {
    println!("Unable to convert path to string: {folder:?}");
  }
}

pub fn set_save_path(storage: &mut dyn Storage, folder: &Path) {
  if let Some(folder) = folder.to_str() {
    set_value(storage, SAVE_PATH_KEY, folder.to_owned());
  } else {
    println!("Unable to convert path to string: {folder:?}");
  }
}

pub fn get_plants(storage: &dyn Storage) -> Option<Vec<Plant>> {
  let text = get_value(storage, PLANTS_KEY)?;
  ron::from_str(&text).ok()
}

pub fn set_plants(storage: &mut dyn Storage, plants: &Vec<Plant>) {
  set_value(storage, PLANTS_KEY, ok!(ron::to_string(plants)));
}

pub fn get_avatar(storage: &dyn Storage) -> Option<String> {
  get_value(storage, AVATAR_KEY)
}

pub fn set_avatar(storage: &mut dyn Storage, avatar: String) {
  set_value(storage, AVATAR_KEY, avatar);
}

pub fn get_notes(storage: &dyn Storage, avatar: &str) -> Option<String> {
  if avatar.is_empty() {
    return None;
  }
  get_value(storage, format!("{avatar} {NOTES_KEY}").as_str())
}

pub fn set_notes(storage: &mut dyn Storage, avatar: &str, notes: String) {
  if !avatar.is_empty() {
    set_value(storage, format!("{avatar} {NOTES_KEY}").as_str(), notes);
  }
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

fn get_value(storage: &dyn Storage, key: &str) -> Option<String> {
  storage.get_string(key)
}

fn set_value(storage: &mut dyn Storage, key: &str, value: String) {
  storage.set_string(key, value);
}
