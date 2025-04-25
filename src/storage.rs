use crate::ok;

use self::inner::{Items, PersistThread};
use std::{
  path::PathBuf,
  sync::{Arc, RwLock},
};

/// Key/value persisted string storage.
#[derive(Clone)]
pub struct Storage {
  items: Arc<RwLock<Items>>,
  thread: Arc<PersistThread>,
}

impl Storage {
  pub fn new(path: PathBuf) -> Option<Self> {
    let items = Arc::new(RwLock::new(Items::load(path)));
    let thread = Arc::new(PersistThread::new(items.clone()));
    Some(Self { items, thread })
  }

  /// Get an item.
  pub fn get(&self, key: &str) -> Option<String> {
    self.items.read().unwrap().get(key).map(|s| s.to_owned())
  }

  /// Get an item as a specific type.
  pub fn get_as<T: serde::de::DeserializeOwned>(&self, key: &str) -> Option<T> {
    let lock = self.items.read().unwrap();
    let text = lock.get(key)?;
    ron::from_str(text).map_err(|e| println!("{e}")).ok()
  }

  // Set an item.
  pub fn set(&mut self, key: &str, item: String) {
    self.items.write().unwrap().set(key, item);
  }

  /// Set an item as a specific type.
  pub fn set_as<T: serde::Serialize>(&mut self, key: &str, item: &T) {
    let text = ok!(ron::to_string(item));
    self.set(key, text);
  }

  /// Remove an item.
  pub fn remove(&mut self, key: &str) {
    self.items.write().unwrap().remove(key);
  }

  /// Persist changes.
  pub fn persist(&self) {
    self.thread.persist();
  }
}

mod inner {
  use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    sync::{
      Arc, RwLock,
      atomic::{AtomicBool, Ordering},
      mpsc::{self, Sender},
    },
    thread::{self, JoinHandle},
  };

  use crate::{err, ok};

  pub struct Items {
    path: PathBuf,
    items: HashMap<String, String>,
    changed: AtomicBool,
  }

  impl Items {
    pub fn load(path: PathBuf) -> Self {
      let items = Self::load_items(&path);
      let changed = AtomicBool::new(false);

      Self { path, items, changed }
    }

    fn load_items(path: &Path) -> HashMap<String, String> {
      let data = ok!(fs::read(path), HashMap::new());
      ok!(ron::de::from_bytes(&data), HashMap::new())
    }

    fn persist(&self) {
      if self.changed.swap(false, Ordering::Relaxed) {
        let text = ok!(ron::ser::to_string_pretty(&self.items, Default::default()));
        err!(fs::write(&self.path, text));
      }
    }

    pub fn get(&self, key: &str) -> Option<&String> {
      self.items.get(key)
    }

    pub fn set(&mut self, key: &str, item: String) {
      let prev = self.items.insert(key.to_owned(), item);
      if self.items.get(key) != prev.as_ref() {
        self.changed.store(true, Ordering::Relaxed);
      }
    }

    pub fn remove(&mut self, key: &str) {
      if self.items.remove(key).is_some() {
        self.changed.store(true, Ordering::Relaxed);
      }
    }
  }

  impl Drop for Items {
    fn drop(&mut self) {
      self.persist();
    }
  }

  pub struct PersistThread {
    thread: Option<JoinHandle<()>>,
    tx: Option<Sender<()>>,
  }

  impl PersistThread {
    pub fn new(items: Arc<RwLock<Items>>) -> Self {
      let (tx, rx) = mpsc::channel();
      Self {
        thread: Some(thread::spawn({
          move || {
            // Wait for a message. Exit when the connection is closed.
            while rx.recv().is_ok() {
              // Persist the items.
              items.read().unwrap().persist();
            }
          }
        })),
        tx: Some(tx),
      }
    }

    pub fn persist(&self) {
      if let Some(tx) = &self.tx {
        tx.send(()).unwrap();
      }
    }
  }

  impl Drop for PersistThread {
    fn drop(&mut self) {
      // Close the connection by dropping the sender.
      drop(self.tx.take().unwrap());

      // Wait for the thread to exit.
      self.thread.take().unwrap().join().unwrap();
    }
  }
}
