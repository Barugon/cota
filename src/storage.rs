use std::{
  collections::HashMap,
  fs::{self, File},
  path::{Path, PathBuf},
  sync::{mpsc, Arc, RwLock},
  thread,
};

struct Items {
  path: PathBuf,
  items: HashMap<String, String>,
}

impl Items {
  fn store(&self) {
    let file = ok!(File::create(&self.path));
    ron::ser::to_writer_pretty(file, &self.items, Default::default()).unwrap();
  }
}

/// Key/value persisted storage.
#[derive(Clone)]
pub struct Storage {
  items: Arc<RwLock<Items>>,
  tx: mpsc::Sender<()>,
}

impl Storage {
  pub fn new(path: PathBuf) -> Option<Self> {
    let items = Self::load(&path);
    let items = Arc::new(RwLock::new(Items { path, items }));
    let (tx, rx) = mpsc::channel();
    thread::spawn({
      let items = items.clone();
      move || {
        // Wait for a message. Exit when the connection is closed.
        while rx.recv().is_ok() {
          // Clear out superfluous requests.
          while rx.try_recv().is_ok() {}

          // Store the items map.
          items.read().unwrap().store();
        }
      }
    });

    Some(Self { items, tx })
  }

  fn load(path: &Path) -> HashMap<String, String> {
    if let Ok(bytes) = fs::read(path) {
      match ron::de::from_bytes(&bytes) {
        Ok(items) => return items,
        Err(err) => println!("{err}"),
      }
    }

    HashMap::new()
  }

  fn store(&self) {
    self.tx.send(()).unwrap();
  }

  pub fn get(&self, key: &str) -> Option<String> {
    let lock = self.items.read().unwrap();
    Some(lock.items.get(key)?.to_owned())
  }

  pub fn set(&mut self, key: &str, item: String) {
    let mut lock = self.items.write().unwrap();
    if let Some(old) = lock.items.insert(key.to_owned(), item) {
      if lock.items.get(key).unwrap() == &old {
        return;
      }
    }

    self.store();
  }

  pub fn remove(&mut self, key: &str) {
    let mut lock = self.items.write().unwrap();
    if lock.items.remove(key).is_some() {
      self.store();
    }
  }
}
