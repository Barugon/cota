use std::{
  collections::HashMap,
  fs::{self, File},
  path::{Path, PathBuf},
  sync::{mpsc, Arc, RwLock},
  thread::{self, JoinHandle},
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

struct ItemsThread {
  thread: Option<JoinHandle<()>>,
  tx: Option<mpsc::Sender<()>>,
}

impl ItemsThread {
  fn store(&self) {
    if let Some(tx) = &self.tx {
      tx.send(()).unwrap();
    }
  }
}

impl Drop for ItemsThread {
  fn drop(&mut self) {
    // Close the connection by dropping the sender.
    drop(self.tx.take().unwrap());

    // Wait for the thread to exit.
    self.thread.take().unwrap().join().unwrap();
  }
}

/// Key/value persisted storage.
#[derive(Clone)]
pub struct Storage {
  items: Arc<RwLock<Items>>,
  thread: Arc<ItemsThread>,
}

impl Storage {
  pub fn new(path: PathBuf) -> Option<Self> {
    let items = Self::load(&path);
    let items = Arc::new(RwLock::new(Items { path, items }));
    let (tx, rx) = mpsc::channel();
    let thread = Some(thread::spawn({
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
    }));

    let tx = Some(tx);
    let thread = Arc::new(ItemsThread { thread, tx });
    Some(Self { items, thread })
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

  /// Get an item.
  pub fn get(&self, key: &str) -> Option<String> {
    let lock = self.items.read().unwrap();
    Some(lock.items.get(key)?.to_owned())
  }

  /// Get an item as a specific type.
  pub fn get_as<T: serde::de::DeserializeOwned>(&self, key: &str) -> Option<T> {
    let lock = self.items.read().unwrap();
    let text = lock.items.get(key)?;
    match ron::from_str(text) {
      Ok(val) => Some(val),
      Err(err) => {
        println!("{err}");
        None
      }
    }
  }

  // Set an item.
  pub fn set(&mut self, key: &str, item: String) {
    let mut lock = self.items.write().unwrap();
    if let Some(old) = lock.items.insert(key.to_owned(), item) {
      if lock.items.get(key).unwrap() == &old {
        return;
      }
    }

    self.thread.store();
  }

  /// Set an item as a specific type.
  pub fn set_as<T: serde::Serialize>(&mut self, key: &str, item: &T) {
    match ron::to_string(item) {
      Ok(text) => self.set(key, text),
      Err(err) => println!("{err}"),
    }
  }

  /// Remove an item.
  pub fn remove(&mut self, key: &str) {
    let mut lock = self.items.write().unwrap();
    if lock.items.remove(key).is_some() {
      self.thread.store();
    }
  }
}
