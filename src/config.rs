use crate::{
  plant_info::Plant,
  util::{Check, Page, APP_NAME},
};
use eframe::epaint::Pos2;
use std::{
  collections::{BTreeSet, HashMap},
  fs::{self, File},
  path::{Path, PathBuf},
  sync::{mpsc, Arc, RwLock},
  thread::{self, JoinHandle},
};

const WINDOW_POS_KEY: &str = "window_pos";
const LOG_PATH_KEY: &str = "log_path";
const SAVE_PATH_KEY: &str = "save_path";
const STATS_AVATAR_KEY: &str = "stats_avatar";
const EXP_AVATAR_KEY: &str = "experience_avatar";
const AVATAR_SKILLS: &str = "skills";
const PLANTS_KEY: &str = "plants";
const DESCRIPTIONS_KEY: &str = "crop_descriptions";
const NOTES_KEY: &str = "notes";
const PAGE_KEY: &str = "page";

struct ItemStore {
  path: PathBuf,
  items: HashMap<String, String>,
}

#[derive(Eq, PartialEq)]
enum Message {
  Persist,
  Exit,
}

impl ItemStore {
  fn persist(&self) {
    let file = ok!(File::create(&self.path));
    ron::ser::to_writer_pretty(file, &self.items, Default::default()).check();
  }
}

struct ItemStoreThread {
  thread: Option<JoinHandle<()>>,
  tx: mpsc::Sender<Message>,
}

impl Drop for ItemStoreThread {
  fn drop(&mut self) {
    self.tx.send(Message::Exit).check();
    if let Some(handle) = self.thread.take() {
      handle.join().check();
    }
  }
}

#[derive(Clone)]
pub struct Config {
  store: Arc<RwLock<ItemStore>>,
  thread: Arc<ItemStoreThread>,
}

impl Config {
  pub fn new() -> Option<Self> {
    let path = Self::path()?;
    let items = Self::load(&path);
    let store = Arc::new(RwLock::new(ItemStore { path, items }));
    let (tx, rx) = mpsc::channel::<Message>();
    let thread = {
      let store = store.clone();
      Some(thread::spawn(move || loop {
        // Wait for a message.
        if rx.recv().check() == Message::Exit {
          return;
        }

        // Only the most recent persist message is needed.
        while let Ok(msg) = rx.try_recv() {
          if msg == Message::Exit {
            return;
          }
        }

        store.read().check().persist();
      }))
    };

    let thread = Arc::new(ItemStoreThread { thread, tx });
    Some(Self { store, thread })
  }

  pub fn get_window_pos(&self) -> Option<Pos2> {
    let lock = self.store.read().check();
    let text = lock.items.get(WINDOW_POS_KEY)?;
    let pos: Option<(f32, f32)> = ok!(ron::from_str(text), None);
    pos.map(|pos| pos.into())
  }

  pub fn set_window_pos(&mut self, pos: Option<Pos2>) {
    let pos: Option<(f32, f32)> = pos.map(|pos| pos.into());
    let text = ok!(ron::to_string(&pos));
    self.set(WINDOW_POS_KEY, text);
  }

  pub fn get_page(&self) -> Option<Page> {
    let lock = self.store.read().check();
    let text = lock.items.get(PAGE_KEY)?;
    Some(ok!(ron::from_str(text), None))
  }

  pub fn set_page(&mut self, page: Page) {
    let text = ok!(ron::to_string(&page));
    self.set(PAGE_KEY, text);
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

    // Remove the entry if notes is empty.
    let key = format!("{avatar} {NOTES_KEY}");
    if notes.is_empty() {
      self.remove(&key);
      return;
    }

    self.set(&key, notes);
  }

  pub fn get_plants(&self) -> Option<Vec<Plant>> {
    let lock = self.store.read().check();
    let text = lock.items.get(PLANTS_KEY)?;
    Some(ok!(ron::from_str(text), None))
  }

  pub fn set_plants(&mut self, plants: &Vec<Plant>) {
    // Remove the entry if plants is empty.
    if plants.is_empty() {
      self.remove(PLANTS_KEY);
      return;
    }

    let text = ok!(ron::to_string(plants));
    self.set(PLANTS_KEY, text);
  }

  pub fn get_crop_descriptions(&self) -> Option<BTreeSet<String>> {
    let lock = self.store.read().check();
    let text = lock.items.get(DESCRIPTIONS_KEY)?;
    Some(ok!(ron::from_str(text), None))
  }

  pub fn set_crop_descriptions(&mut self, descriptions: &BTreeSet<String>) {
    // Remove the entry if the set is empty.
    if descriptions.is_empty() {
      self.remove(DESCRIPTIONS_KEY);
      return;
    }

    let text = ok!(ron::to_string(descriptions));
    self.set(DESCRIPTIONS_KEY, text);
  }

  pub fn get_avatar_skills(&self, avatar: &str) -> Option<HashMap<u32, (i32, i32)>> {
    if avatar.is_empty() {
      return None;
    }

    let key = format!("{avatar} {AVATAR_SKILLS}");
    let lock = self.store.read().check();
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

    // Remove the entry if skills is empty.
    let key = format!("{avatar} {AVATAR_SKILLS}");
    if skills.is_empty() {
      self.remove(&key);
      return;
    }

    let text = ok!(ron::to_string(&skills));
    self.set(&key, text);
  }

  fn path() -> Option<PathBuf> {
    dirs::config_dir().map(|path| path.join(APP_NAME).with_extension("ron"))
  }

  fn load(path: &Path) -> HashMap<String, String> {
    let Ok(bytes) = fs::read(path) else { return HashMap::new() };
    ok!(ron::de::from_bytes(&bytes), HashMap::new())
  }

  fn get(&self, key: &str) -> Option<String> {
    let lock = self.store.read().check();
    Some(lock.items.get(key)?.to_owned())
  }

  fn set(&mut self, key: &str, item: String) {
    let mut lock = self.store.write().check();
    lock.items.insert(key.to_owned(), item);
    self.persist();
  }

  fn remove(&mut self, key: &str) {
    let mut lock = self.store.write().check();
    if lock.items.remove(key).is_some() {
      self.persist();
    }
  }

  fn persist(&self) {
    self.thread.tx.send(Message::Persist).check();
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
