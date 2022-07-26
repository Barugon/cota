use crate::util;
use serde_json::Value;
use std::{borrow::Cow, fs::File, io::Write, path::PathBuf, sync::RwLock};

/// Structure to load and modify a SotA save-game file.
pub struct GameData {
  // Save file path.
  path: RwLock<PathBuf>,

  // Full file text.
  text: String,

  // Avatar ID.
  avatar: String,

  // Parsed JSON sections.
  character: Value,
  gold: Value,

  // Save date.
  date: Value,
}

impl GameData {
  pub fn load(path: PathBuf) -> Result<Self, Cow<'static, str>> {
    match std::fs::read_to_string(&path) {
      Ok(text) => {
        // Get the avatar ID.
        let err = Err(Cow::from("Unable to determine the current avatar"));
        let avatar = some!(get_avatar_id(&text), err);

        // Get the 'UserGold' json.
        let err = Err(Cow::from("Unable to find user gold"));
        let gold = some!(get_json(&text, "UserGold", USER_ID), err);
        if !gold.is_object() {
          return Err(Cow::from("Error reading user gold"));
        }

        // Get the 'CharacterSheet' JSON.
        let err = Err(Cow::from("Unable to find character sheet"));
        let character = some!(get_json(&text, "CharacterSheet", &avatar), err);
        if !character.is_object() {
          return Err(Cow::from("Error reading character sheet"));
        }

        // Make sure adventurer experience is there.
        let err = Err(Cow::from("Unable to parse adventurer experience"));
        some!(some!(character.get(AE), err).to_i64(), err);

        // Get the skills value.
        let err = Err(Cow::from("Unable to find skills"));
        let skills = some!(character.get(SK2), err);
        if !skills.is_object() {
          return Err(Cow::from("Error reading skills"));
        }

        // Find a date.
        let err = Err(Cow::from("Unable to parse the date/time"));
        let date = some!(find_date(skills), err);

        Ok(GameData {
          path: RwLock::new(path),
          text,
          avatar,
          character,
          gold,
          date,
        })
      }
      Err(err) => Err(Cow::from(format!("Unable to load file: {}", err))),
    }
  }

  pub fn store(&self) -> Result<(), Cow<'static, str>> {
    let path = self.path.read().unwrap().clone();
    self.store_as(path)
  }

  pub fn store_as(&self, path: PathBuf) -> Result<(), Cow<'static, str>> {
    // Set UserGold.
    let err = Err(Cow::from("Unable to set UserGold"));
    let text = some!(set_json(&self.text, "UserGold", USER_ID, &self.gold), err);

    // Set CharacterSheet.
    let err = Err(Cow::from("Unable to set CharacterSheet"));
    let text = some!(
      set_json(&text, "CharacterSheet", &self.avatar, &self.character),
      err
    );

    // Create the save-game file and store the data.
    match File::create(&path) {
      Ok(mut file) => match file.write_all(text.as_bytes()) {
        Ok(()) => {
          // Change the path.
          *self.path.write().unwrap() = path;
          Ok(())
        }
        Err(err) => Err(Cow::from(format!("Unable to store file: {}", err))),
      },
      Err(err) => Err(Cow::from(format!("Unable to store file: {}", err))),
    }
  }

  pub fn get_gold(&self) -> Option<i64> {
    let gold = self.gold.get(G)?;
    gold.to_i64()
  }

  pub fn set_gold(&mut self, gold: i64) {
    self.gold[G] = gold.into();
  }

  pub fn get_skill_lvl(&self, id: u64) -> Option<i32> {
    get_skill_lvl(self.character.get(SK2).unwrap(), id)
  }

  pub fn set_skill_lvl(&mut self, id: u64, lvl: i32) {
    if lvl == 0 {
      self.remove_skill(id)
    } else {
      assert!(util::LVL_RANGE.contains(&lvl));
      self.set_skill_exp(id, util::SKILL_EXP[lvl as usize - 1]);
    }
  }

  pub fn get_adv_lvl(&self) -> i32 {
    let exp = self.get_adv_exp();
    find_min(exp, &util::LEVEL_EXP).unwrap() as i32 + 1
  }

  pub fn set_adv_lvl(&mut self, lvl: i32) {
    assert!(util::LVL_RANGE.contains(&lvl));
    self.set_adv_exp(util::LEVEL_EXP[lvl as usize - 1]);
  }

  pub fn get_file_path(&self) -> PathBuf {
    self.path.read().unwrap().clone()
  }

  fn set_skill_exp(&mut self, id: u64, exp: i64) {
    assert!(exp > 0);
    let key = format!("{}", id);
    let skills = self.character.get_mut(SK2).unwrap();
    if let Some(skill) = skills.get_mut(&key) {
      skill[X] = exp.into();
    } else {
      skills[key] = serde_json::json!({
        M: 0,
        T: self.date,
        X: exp,
      });
    }
  }

  fn remove_skill(&mut self, id: u64) {
    let skills = self.character.get_mut(SK2).unwrap();
    let skills = skills.as_object_mut().unwrap();
    skills.remove(&format!("{}", id));
  }

  fn set_adv_exp(&mut self, exp: i64) {
    self.character[AE] = exp.into();
  }

  fn get_adv_exp(&self) -> i64 {
    self.character.get(AE).unwrap().to_i64().unwrap()
  }
}

pub fn get_skill_lvl(skills: &Value, id: u64) -> Option<i32> {
  let exp = get_skill_exp(skills, id)?;
  let idx = find_min(exp, &util::SKILL_EXP)?;
  Some(idx as i32 + 1)
}

fn get_skill_exp(skills: &Value, id: u64) -> Option<i64> {
  let skill = skills.get(format!("{}", id))?;
  let exp = skill.get(X)?;
  exp.to_i64()
}

trait ToI64 {
  fn to_i64(&self) -> Option<i64>;
}

impl ToI64 for Value {
  fn to_i64(&self) -> Option<i64> {
    match self {
      Value::Number(val) => val.as_i64(),
      Value::String(text) => text.parse().ok(),
      _ => None,
    }
  }
}

const USER_ID: &str = "000000000000000000000001";
const USER: &str = "User";
const SK2: &str = "sk2";
const AE: &str = "ae";
const DC: &str = "dc";
const G: &str = "g";
const M: &str = "m";
const T: &str = "t";
const X: &str = "x";

fn find_min<T: Ord>(value: T, values: &[T]) -> Option<usize> {
  match values.binary_search(&value) {
    Ok(idx) => Some(idx),
    Err(idx) => {
      if idx > 0 {
        Some(idx - 1)
      } else {
        None
      }
    }
  }
}

fn get_avatar_id(text: &str) -> Option<String> {
  // Get the User json.
  let json = get_json(text, USER, USER_ID)?;

  // Get the avatar ID.
  if let Some(Value::String(id)) = json.get(DC) {
    return Some(id.clone());
  }
  None
}

fn collection_tag(collection: &str) -> String {
  format!(r#"<collection name="{}">"#, collection)
}

fn record_tag(id: &str) -> String {
  format!(r#"<record Id="{}">"#, id)
}

const fn record_end() -> &'static str {
  "</record>"
}

fn get_json(text: &str, collection: &str, id: &str) -> Option<Value> {
  // Find the collection tag.
  let find = collection_tag(collection);
  let pos = text.find(&find)?;
  let text = &text[pos + find.len()..];

  // From that point, find the record tag.
  let find = record_tag(id);
  let pos = text.find(&find)?;
  let text = &text[pos + find.len()..];

  // Find the record end tag.
  let pos = text.find(record_end())?;
  let text = &text[..pos];

  // Parse the JSON text.
  match serde_json::from_str(text) {
    Ok(json) => Some(json),
    Err(err) => {
      println!("{:?}", err);
      None
    }
  }
}

fn set_json(text: &str, collection: &str, id: &str, val: &Value) -> Option<String> {
  // Find the collection tag.
  let find = collection_tag(collection);
  let start = text.find(&find)? + find.len();
  let slice = &text[start..];

  // From that point, find the record tag.
  let find = record_tag(id);
  let pos = slice.find(&find)? + find.len();
  let slice = &slice[pos..];
  let start = start + pos;

  // Find the record end tag.
  let pos = slice.find(record_end())?;
  let end = start + pos;

  // Convert the value to JSON text.
  let json = val.to_string();

  // Concatenate the XML with the new JSON.
  let parts = [&text[..start], &json, &text[end..]];
  let mut result = String::new();
  result.reserve(parts[0].len() + parts[1].len() + parts[2].len());
  result.push_str(parts[0]);
  result.push_str(parts[1]);
  result.push_str(parts[2]);
  Some(result)
}

fn find_date(val: &Value) -> Option<Value> {
  if let Value::Object(obj) = val {
    for (_, val) in obj {
      if let Some(val) = val.get(T) {
        return Some(val.clone());
      }
    }
  }
  None
}
