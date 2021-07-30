use gdnative::api::*;
use gdnative::object::{AssumeSafeLifetime, LifetimeConstraint};
use gdnative::prelude::*;
use num_format::Locale;
use std::cmp::Ordering;

#[macro_export]
macro_rules! some {
  ($opt:expr) => {
    match $opt {
      Some(val) => val,
      None => return,
    }
  };
  ($opt:expr, $ret:expr) => {
    match $opt {
      Some(val) => val,
      None => return $ret,
    }
  };
  ($opt:expr, $msg:expr, $ret:expr) => {
    match $opt {
      Some(val) => val,
      None => {
        godot_print!("{}", $msg);
        return $ret;
      }
    }
  };
}

#[macro_export]
macro_rules! ok {
  ($res:expr) => {
    match $res {
      Ok(val) => val,
      Err(err) => {
        godot_print!("{}", err);
        return;
      }
    }
  };
  ($res:expr, $ret:expr) => {
    match $res {
      Ok(val) => val,
      Err(err) => {
        godot_print!("{}", err);
        return $ret;
      }
    }
  };
}

pub struct Cycle<T> {
  index: usize,
  values: Vec<T>,
}

impl<T> Cycle<T> {
  pub fn new(values: Vec<T>) -> Self {
    assert!(!values.is_empty());
    Self { index: 0, values }
  }

  pub fn get(&mut self) -> &T {
    let index = self.index;
    self.index += 1;
    if self.index >= self.values.len() {
      self.index = 0;
    }

    &self.values[index]
  }
}

pub trait OptionButtonText {
  fn find_item_index(self, text: &GodotString) -> Option<i64>;
  fn select_item(self, text: &GodotString) -> bool;
}

impl OptionButtonText for TRef<'_, OptionButton> {
  fn find_item_index(self, text: &GodotString) -> Option<i64> {
    let count = self.get_item_count();
    for index in 0..count {
      let item_text = self.get_item_text(index);
      if item_text == *text {
        return Some(index);
      }
    }
    None
  }

  fn select_item(self, text: &GodotString) -> bool {
    if let Some(index) = self.find_item_index(text) {
      self.select(index);
      return true;
    }
    false
  }
}

pub trait ToRef<'a, 'r, T: GodotObject> {
  fn to_ref(&'r self) -> TRef<'a, T, Shared>;
}

impl<'a, 'r, T: GodotObject> ToRef<'a, 'r, T> for Ref<T, Shared>
where
  AssumeSafeLifetime<'a, 'r>: LifetimeConstraint<<T>::RefKind>,
{
  fn to_ref(&'r self) -> TRef<'a, T, Shared> {
    unsafe { self.assume_safe() }
  }
}

pub trait GetNodeAs {
  fn get_node_as<T>(self, path: &GodotString) -> Option<TRef<'_, T, Shared>>
  where
    T: GodotObject + SubClass<Node>;
}

impl GetNodeAs for TRef<'_, Node> {
  fn get_node_as<T>(self, path: &GodotString) -> Option<TRef<'_, T, Shared>>
  where
    T: GodotObject + SubClass<Node>,
  {
    if let Some(node) = self.get_node(NodePath::new(path)) {
      let node = node.to_ref().cast();
      if node.is_none() {
        let t = std::any::type_name::<T>();
        godot_print!("Unable to cast node {} as {}", path, t);
      }
      return node;
    } else {
      godot_print!("Unable to get node {}", path);
    }
    None
  }
}

pub trait Method {
  fn method(self, path: &GodotString, method: &GodotString, args: &[Variant]) -> Variant;
  fn method_deferred(self, path: &GodotString, method: &GodotString, args: &[Variant]) -> Variant;
}

impl Method for TRef<'_, Node> {
  fn method(self, path: &GodotString, method: &GodotString, args: &[Variant]) -> Variant {
    if let Some(node) = self.get_node(NodePath::new(path)) {
      unsafe {
        return node.assume_safe().call(method.clone(), args);
      }
    }
    Variant::new()
  }

  fn method_deferred(self, path: &GodotString, method: &GodotString, args: &[Variant]) -> Variant {
    if let Some(node) = self.get_node(NodePath::new(path)) {
      unsafe {
        return node.assume_safe().call_deferred(method.clone(), args);
      }
    }
    Variant::new()
  }
}

pub trait ConnectTo {
  fn connect_to(self, path: &GodotString, signal: &str, slot: &str) -> bool;
}

impl ConnectTo for TRef<'_, Node> {
  fn connect_to(self, path: &GodotString, signal: &str, slot: &str) -> bool {
    if let Some(node) = self.get_node(NodePath::new(path)) {
      let mut node = node.to_ref();

      // Get the popup if this is a menu button.
      if let Some(button) = node.cast::<MenuButton>() {
        if let Some(popup) = button.get_popup() {
          node = popup.to_ref().upcast::<Node>();
        } else {
          godot_print!("Unable to get popup for {}", path);
          return false;
        }
      }

      if let Err(err) = node.connect(signal, self, slot, VariantArray::new_shared(), 0) {
        godot_print!("Unable to connect {}: {}", slot, err);
      } else {
        return true;
      }
    } else {
      godot_print!("Unable to get node {}", path);
    }
    false
  }
}

pub trait SetShortcut {
  fn set_shortcut(self, id: i64, key: i64, ctrl: bool);
}

impl SetShortcut for TRef<'_, PopupMenu> {
  fn set_shortcut(self, id: i64, key: i64, ctrl: bool) {
    let input = InputEventKey::new();
    input.set_control(ctrl);
    input.set_scancode(key);
    self.set_item_accelerator(self.get_item_index(id), input.get_scancode_with_modifiers());
  }
}

pub trait Get {
  fn get(&self, key: &Variant) -> Option<Variant>;
}

impl Get for Variant {
  fn get(&self, key: &Variant) -> Option<Variant> {
    if let Some(dictionary) = self.try_to_dictionary() {
      return Some(dictionary.get(key));
    }
    None
  }
}

impl Get for Option<Variant> {
  fn get(&self, key: &Variant) -> Option<Variant> {
    if let Some(variant) = self {
      return variant.get(key);
    }
    None
  }
}

pub trait Set {
  fn set(&mut self, key: &Variant, value: &Variant) -> bool;
}

impl Set for Variant {
  fn set(&mut self, key: &Variant, value: &Variant) -> bool {
    if let Some(dictionary) = self.try_to_dictionary() {
      unsafe { dictionary.assume_unique() }.insert(key, value);
      return true;
    }
    false
  }
}

impl Set for Option<Variant> {
  fn set(&mut self, key: &Variant, value: &Variant) -> bool {
    if let Some(variant) = self {
      return variant.set(key, value);
    }
    false
  }
}

pub trait Erase {
  fn erase(&mut self, key: &Variant);
}

impl Erase for Variant {
  fn erase(&mut self, key: &Variant) {
    if let Some(dictionary) = self.try_to_dictionary() {
      unsafe { dictionary.assume_unique() }.erase(key);
    }
  }
}

impl Erase for Option<Variant> {
  fn erase(&mut self, key: &Variant) {
    if let Some(variant) = self {
      return variant.erase(key);
    }
  }
}

pub trait ToText {
  fn to_text(&self) -> Option<GodotString>;
}

impl ToText for Option<Variant> {
  fn to_text(&self) -> Option<GodotString> {
    if let Some(variant) = self {
      return Some(variant.to_godot_string());
    }
    None
  }
}

pub trait ToInt {
  fn to_int(&self) -> Option<i64>;
}

impl ToInt for Option<Variant> {
  fn to_int(&self) -> Option<i64> {
    if let Some(variant) = self {
      return Some(variant.to_i64());
    }
    None
  }
}

pub struct Config {
  log_path: Option<GodotString>,
  cfg_path: GodotString,
  main: GodotString,
  _items: GodotString,
  folder_key: GodotString,
  avatar_key: GodotString,
  notes_key: GodotString,
}

impl Config {
  pub fn new() -> Config {
    let mut log_path = None;
    if let Some(dir) = dirs::config_dir() {
      let path = dir.join("Portalarium/Shroud of the Avatar/ChatLogs");
      if let Some(path) = path.to_str() {
        let path = if cfg!(target_os = "windows") {
          // Change any backslashes to forward slashes.
          GodotString::from(path.replace('\\', "/"))
        } else {
          GodotString::from(path)
        };
        log_path = Some(path);
      }
    }

    Config {
      log_path,
      cfg_path: GodotString::from("user://settings.cfg"),
      main: GodotString::from("main"),
      _items: GodotString::from("items"),
      folder_key: GodotString::from("log_folder"),
      avatar_key: GodotString::from("avatar"),
      notes_key: GodotString::from("notes"),
    }
  }

  pub fn get_log_folder(&self) -> Option<GodotString> {
    if let Some(folder) = self.get_value(&self.main, &self.folder_key) {
      godot_print!("Log folder = {}", folder);
      return Some(folder);
    } else if let Some(folder) = &self.log_path {
      godot_print!("Log folder = {}", folder);
      return Some(folder.clone());
    }
    None
  }

  pub fn set_log_folder(&self, folder: Option<&GodotString>) {
    self.set_value(&self.main, &self.folder_key, folder);
  }

  pub fn get_avatar(&self) -> Option<GodotString> {
    self.get_value(&self.main, &self.avatar_key)
  }

  pub fn set_avatar(&self, avatar: Option<&GodotString>) {
    self.set_value(&self.main, &self.avatar_key, avatar);
  }

  pub fn get_notes(&self, avatar: &GodotString) -> Option<GodotString> {
    if !avatar.is_empty() {
      return self.get_value(avatar, &self.notes_key);
    }
    None
  }

  pub fn set_notes(&self, avatar: &GodotString, notes: Option<&GodotString>) {
    if !avatar.is_empty() {
      self.set_value(avatar, &self.notes_key, notes);
    }
  }

  pub fn _get_items(&self) -> Vec<(GodotString, i64)> {
    let mut items = Vec::new();
    let config = self.load();
    if config.has_section(self._items.clone()) {
      let names = config.get_section_keys(self._items.clone());
      for index in 0..names.len() {
        let name = names.get(index);
        if !name.is_empty() {
          let id = config.get_value(self._items.clone(), name.clone(), Variant::new());
          if !id.is_nil() {
            let id = id.to_i64();
            if id != 0 {
              items.push((name, id));
            }
          }
        }
      }
    }
    items
  }

  pub fn _add_item(&self, name: GodotString, id: i64) {
    let config = self.load();
    config.set_value(self._items.clone(), name, Variant::from_i64(id));
    self.save(config);
  }

  fn get_value(&self, section: &GodotString, key: &GodotString) -> Option<GodotString> {
    let config = self.load();
    if config.has_section_key(section.clone(), key.clone()) {
      let value = config.get_value(section.clone(), key.clone(), Variant::new());
      if !value.is_nil() {
        return Some(value.to_godot_string());
      }
    }
    None
  }

  fn set_value(&self, section: &GodotString, key: &GodotString, value: Option<&GodotString>) {
    let config = self.load();
    if let Some(value) = value {
      let var = Variant::from_godot_string(value);
      config.set_value(section.clone(), key.clone(), var);
    } else if config.has_section_key(section.clone(), key.clone()) {
      config.erase_section_key(section.clone(), key.clone());
    }
    self.save(config);
  }

  fn load(&self) -> Ref<ConfigFile, Unique> {
    let config = ConfigFile::new();
    if !self.cfg_path.is_empty() {
      let _ = config.load(self.cfg_path.clone());
    }
    config
  }

  fn save(&self, config: Ref<ConfigFile, Unique>) {
    if !self.cfg_path.is_empty() {
      if let Err(err) = config.save(self.cfg_path.clone()) {
        godot_print!("Unable to save config: {}", err);
      }
    } else {
      godot_print!("Config file path is empty");
    }
  }
}

pub fn ascii_starts_with_ignore_case(container: &[u8], pattern: &[u8]) -> bool {
  if pattern.is_empty() || container.len() < pattern.len() {
    return false;
  }

  for index in 0..pattern.len() {
    if container[index].to_ascii_lowercase() != pattern[index].to_ascii_lowercase() {
      return false;
    }
  }

  true
}

pub fn ascii_contains_ignore_case(container: &[u8], pattern: &[u8]) -> bool {
  if !pattern.is_empty() {
    let mut container = container;
    while container.len() >= pattern.len() {
      if ascii_starts_with_ignore_case(container, pattern) {
        return true;
      }
      container = &container[1..];
    }
  }

  false
}

pub fn ascii_compare_ignore_case(left: &[u8], right: &[u8]) -> Ordering {
  let mut il = left.iter();
  let mut ir = right.iter();
  loop {
    if let Some(cl) = il.next() {
      if let Some(cr) = ir.next() {
        match cl.to_ascii_lowercase().cmp(&cr.to_ascii_lowercase()) {
          Ordering::Less => return Ordering::Less,
          Ordering::Equal => continue,
          Ordering::Greater => return Ordering::Greater,
        }
      }
    }
    return left.len().cmp(&right.len());
  }
}

pub fn get_locale() -> Locale {
  let names = Locale::available_names();
  let name = OS::godot_singleton().get_locale();
  let name = name.to_utf8().as_str().replace('_', "-");

  // Search for an exact match.
  if let Ok(pos) =
    names.binary_search_by(|n| ascii_compare_ignore_case(n.as_bytes(), name.as_bytes()))
  {
    if let Ok(locale) = Locale::from_name(names[pos]) {
      return locale;
    }
  } else {
    // Exact match not found, try the base language.
    if let Some(name) = name.split('-').next() {
      if let Ok(locale) = Locale::from_name(name) {
        return locale;
      }
    }
  }

  Locale::en
}

pub trait ToDisplayString {
  fn to_display_string(&self, locale: Locale) -> String;
}

impl ToDisplayString for f64 {
  fn to_display_string(&self, locale: Locale) -> String {
    format!("{:.6}", self)
      .trim_end_matches('0')
      .trim_end_matches('.')
      .replacen('.', locale.decimal(), 1)
  }
}
