use std::{
  collections::HashMap,
  fs::{self, File},
  path::PathBuf,
  sync::{
    atomic::{AtomicBool, Ordering},
    mpsc, Arc, RwLock,
  },
  thread::{self, JoinHandle},
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
    Some(self.items.read().unwrap().get(key)?.to_owned())
  }

  /// Get an item as a specific type.
  pub fn get_as<T: serde::de::DeserializeOwned>(&self, key: &str) -> Option<T> {
    let lock = self.items.read().unwrap();
    let text = lock.get(key)?;
    match ron::from_str(text) {
      Ok(ok) => Some(ok),
      Err(err) => {
        println!("{err}");
        None
      }
    }
  }

  // Set an item.
  pub fn set(&mut self, key: &str, item: String) {
    self.items.write().unwrap().set(key, item);
  }

  /// Set an item as a specific type.
  pub fn set_as<T: serde::Serialize>(&mut self, key: &str, item: &T) {
    match ron::to_string(item) {
      Ok(ok) => self.set(key, ok),
      Err(err) => println!("{err}"),
    }
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

struct Items {
  path: PathBuf,
  items: HashMap<String, String>,
  changed: AtomicBool,
}

impl Items {
  fn load(path: PathBuf) -> Self {
    let changed = AtomicBool::new(false);
    let mut items = HashMap::new();
    if let Ok(bytes) = fs::read(&path) {
      match ron::de::from_bytes(&bytes) {
        Ok(ok) => items = ok,
        Err(err) => println!("{err}"),
      }
    }

    Self {
      path,
      items,
      changed,
    }
  }

  fn persist(&self) {
    if self.changed.swap(false, Ordering::Relaxed) {
      let file = ok!(File::create(&self.path));
      ron::ser::to_writer_pretty(file, &self.items, Default::default()).unwrap();
    }
  }

  fn get(&self, key: &str) -> Option<&str> {
    self.items.get(key).map(|s| s.as_str())
  }

  fn set(&mut self, key: &str, item: String) {
    if let Some(prev) = self.items.insert(key.to_owned(), item) {
      if self.items.get(key).unwrap() != &prev {
        self.changed.store(true, Ordering::Relaxed);
      }
    }
  }

  fn remove(&mut self, key: &str) {
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

struct PersistThread {
  thread: Option<JoinHandle<()>>,
  tx: Option<mpsc::Sender<()>>,
}

impl PersistThread {
  fn new(items: Arc<RwLock<Items>>) -> Self {
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

  fn persist(&self) {
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
