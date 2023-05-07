use crate::{
  plant_info::Plant,
  util::{APP_NAME, FAIL_ERR},
};
use eframe::epaint::Pos2;
use std::{
  collections::HashMap,
  fs::File,
  io::BufReader,
  path::{Path, PathBuf},
  sync::{Arc, RwLock},
};

const WINDOW_POS_KEY: &str = "window_pos";
const LOG_PATH_KEY: &str = "log_path";
const SAVE_PATH_KEY: &str = "save_path";
const STATS_AVATAR_KEY: &str = "stats_avatar";
const EXP_AVATAR_KEY: &str = "experience_avatar";
const AVATAR_SKILLS: &str = "skills";
const PLANTS_KEY: &str = "plants";
const NOTES_KEY: &str = "notes";

struct ItemStore {
  path: PathBuf,
  items: HashMap<String, String>,
  modified: bool,
}

impl ItemStore {
  fn store(&mut self) {
    if self.modified {
      let file = File::create(&self.path).expect(FAIL_ERR);
      ron::ser::to_writer_pretty(file, &self.items, Default::default()).expect(FAIL_ERR);
      self.modified = false;
    }
  }
}

impl Drop for ItemStore {
  fn drop(&mut self) {
    self.store();
  }
}

#[derive(Clone)]
pub struct Config {
  store: Arc<RwLock<ItemStore>>,
}

impl Config {
  pub fn new() -> Option<Self> {
    let path = Self::path()?;
    let items = Self::load(&path);
    let store = ItemStore {
      path,
      items,
      modified: false,
    };

    Some(Self {
      store: Arc::new(RwLock::new(store)),
    })
  }

  pub fn get_window_pos(&self) -> Option<Pos2> {
    let lock = self.store.read().expect(FAIL_ERR);
    let text = lock.items.get(WINDOW_POS_KEY)?;
    let pos: Option<(f32, f32)> = ok!(ron::from_str(text), None);
    pos.map(|pos| pos.into())
  }

  pub fn set_window_pos(&mut self, pos: Option<Pos2>) {
    let pos: Option<(f32, f32)> = pos.map(|pos| pos.into());
    let text = ok!(ron::to_string(&pos));
    self.set(WINDOW_POS_KEY, text);
  }

  pub fn get_log_path(&self) -> Option<PathBuf> {
    if let Some(folder) = self.get(LOG_PATH_KEY) {
      return Some(PathBuf::from(folder));
    }
    Self::get_default_log_path()
  }

  pub fn set_log_path(&mut self, folder: &Path) {
    if let Some(folder) = folder.to_str() {
      self.set(LOG_PATH_KEY, folder.to_owned());
    } else {
      println!("Unable to convert path to string: {folder:?}");
    }
  }

  pub fn get_save_path(&self) -> Option<PathBuf> {
    if let Some(folder) = self.get(SAVE_PATH_KEY) {
      return Some(PathBuf::from(folder));
    }
    Self::get_default_save_path()
  }

  pub fn set_save_path(&mut self, folder: &Path) {
    if let Some(folder) = folder.to_str() {
      self.set(SAVE_PATH_KEY, folder.to_owned());
    } else {
      println!("Unable to convert path to string: {folder:?}");
    }
  }

  pub fn get_stats_avatar(&self) -> Option<String> {
    self.get(STATS_AVATAR_KEY)
  }

  pub fn set_stats_avatar(&mut self, avatar: String) {
    if avatar.is_empty() {
      return;
    }

    self.set(STATS_AVATAR_KEY, avatar);
  }

  pub fn get_exp_avatar(&self) -> Option<String> {
    self.get(EXP_AVATAR_KEY)
  }

  pub fn set_exp_avatar(&mut self, avatar: String) {
    if avatar.is_empty() {
      return;
    }

    self.set(EXP_AVATAR_KEY, avatar);
  }

  pub fn get_notes(&self, avatar: &str) -> Option<String> {
    if avatar.is_empty() {
      return None;
    }

    let key = format!("{avatar} {NOTES_KEY}");
    self.get(&key)
  }

  pub fn set_notes(&mut self, avatar: &str, notes: String) {
    if avatar.is_empty() {
      return;
    }

    let key = format!("{avatar} {NOTES_KEY}");
    self.set(&key, notes);
  }

  pub fn get_plants(&self) -> Option<Vec<Plant>> {
    let lock = self.store.read().expect(FAIL_ERR);
    let text = lock.items.get(PLANTS_KEY)?;
    Some(ok!(ron::from_str(text), None))
  }

  pub fn set_plants(&mut self, plants: &Vec<Plant>) {
    let text = ok!(ron::to_string(plants));
    self.set(PLANTS_KEY, text);
  }

  pub fn get_avatar_skills(&self, avatar: &str) -> Option<HashMap<u32, (i32, i32)>> {
    if avatar.is_empty() {
      return None;
    }

    let key = format!("{avatar} {AVATAR_SKILLS}");
    let lock = self.store.read().expect(FAIL_ERR);
    let text = lock.items.get(&key)?;
    Some(ok!(ron::from_str(text), None))
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

    let text = ok!(ron::to_string(&skills));
    let key = format!("{avatar} {AVATAR_SKILLS}");
    self.set(&key, text);
  }

  fn get(&self, key: &str) -> Option<String> {
    let lock = self.store.read().expect(FAIL_ERR);
    Some(lock.items.get(key)?.to_owned())
  }

  fn set(&mut self, key: &str, item: String) {
    let mut lock = self.store.write().expect(FAIL_ERR);
    lock.items.insert(key.to_owned(), item);
    lock.modified = true;
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

  fn load(path: &Path) -> HashMap<String, String> {
    let Some(file) = File::open(path).ok() else { return HashMap::new() };
    let reader = BufReader::new(file);
    ok!(ron::de::from_reader(reader), HashMap::new())
  }
}
