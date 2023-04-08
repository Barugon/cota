use crate::{plant_info::Plant, skill_info::SkillLvlPlan};
use eframe::Storage;
use std::{
  collections::HashMap,
  path::{Path, PathBuf},
};

const LOG_PATH_KEY: &str = "log_path";
const SAVE_PATH_KEY: &str = "save_path";
const SKILL_LEVELS_KEY: &str = "skill_levels";
const STATS_AVATAR_KEY: &str = "stats_avatar";
const EXP_AVATAR_KEY: &str = "experience_avatar";
const PLANTS_KEY: &str = "plants";
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

pub fn get_stats_avatar(storage: &dyn Storage) -> Option<String> {
  get_value(storage, STATS_AVATAR_KEY)
}

pub fn set_stats_avatar(storage: &mut dyn Storage, avatar: String) {
  set_value(storage, STATS_AVATAR_KEY, avatar);
}

pub fn get_exp_avatar(storage: &dyn Storage) -> Option<String> {
  get_value(storage, EXP_AVATAR_KEY)
}

pub fn set_exp_avatar(storage: &mut dyn Storage, avatar: String) {
  set_value(storage, EXP_AVATAR_KEY, avatar);
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

pub fn get_plants(storage: &dyn Storage) -> Option<Vec<Plant>> {
  let text = get_value(storage, PLANTS_KEY)?;
  Some(ok!(ron::from_str(&text), None))
}

pub fn set_plants(storage: &mut dyn Storage, plants: &Vec<Plant>) {
  let text = ok!(ron::to_string(plants));
  set_value(storage, PLANTS_KEY, text);
}

pub fn get_levels(storage: &mut dyn Storage, avatar: &str) -> Option<HashMap<u32, SkillLvlPlan>> {
  if avatar.is_empty() {
    return None;
  }

  let text = get_value(storage, format!("{avatar} {SKILL_LEVELS_KEY}").as_str())?;
  Some(ok!(ron::from_str(&text), None))
}

pub fn set_levels(storage: &mut dyn Storage, avatar: &str, levels: &HashMap<u32, SkillLvlPlan>) {
  if avatar.is_empty() {
    return;
  }

  // Filter out empties.
  let levels: HashMap<u32, SkillLvlPlan> = levels
    .iter()
    .filter(|(_, plan)| plan.cur > 0 && plan.tgt > 0)
    .map(|(id, plan)| (*id, *plan))
    .collect();

  let text = ok!(ron::to_string(&levels));
  let key = format!("{avatar} {SKILL_LEVELS_KEY}");
  set_value(storage, &key, text);
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
