use crate::util::{self, SkillCategory, SkillInfo, SkillInfoGroup};
use serde_json::Value;
use std::{borrow::Cow, fs::File, io::Write, path::PathBuf, sync::RwLock};

// NOTE: UserKnowledge contains virtue.

/// Structure to load and modify a SotA save-game file.
pub struct GameData {
  // Save file path.
  path: RwLock<PathBuf>,

  // Full file text.
  text: String,

  // Avatar ID.
  avatar: String,

  // Avatar name.
  name: String,

  // Backpack ID.
  backpack: String,

  // Parsed JSON sections.
  character: Value,
  inventory: Value,
  gold: Value,

  // Save date.
  date: Value,
}

impl GameData {
  pub fn load(path: PathBuf) -> Result<Self, Cow<'static, str>> {
    match std::fs::read_to_string(&path) {
      Ok(text) => {
        // Get the avatar ID.
        let Some(avatar) = get_avatar_id(&text) else { return Err(Cow::from("Unable to determine the current avatar")) };

        // Get the avatar name.
        let Some(name) = get_avatar_name(&text, &avatar) else { return Err(Cow::from("Unable to get the avatar name")) };

        // Get the CharacterSheet JSON.
        let Some(character) = get_json(&text, "CharacterSheet", &avatar) else { return Err(Cow::from("Unable to find character sheet")) };
        if !character.is_object() {
          return Err(Cow::from("Error reading character sheet"));
        }

        // Get the backpack ID.
        let Some(backpack) = get_backpack_id(&text, &avatar) else { return Err(Cow::from("Unable to find the avatar's backpack")) };

        // Get the ItemStore JSON.
        let Some(inventory) = get_json(&text, "ItemStore", &backpack) else { return Err(Cow::from("Unable to find inventory")) };
        if !inventory.is_object() {
          return Err(Cow::from("Error reading inventory"));
        }

        // Get the UserGold json.
        let Some(gold) = get_json(&text, "UserGold", USER_ID) else { return Err(Cow::from("Unable to find user gold")) };
        if !gold.is_object() {
          return Err(Cow::from("Error reading user gold"));
        }

        // Make sure adventurer experience is there.
        if character.get(AE).and_then(|exp| exp.to_i64()).is_none() {
          return Err(Cow::from("Unable to parse adventurer experience"));
        }

        // Make sure producer experience is there.
        if character.get(PE).and_then(|exp| exp.to_i64()).is_none() {
          return Err(Cow::from("Unable to parse producer experience"));
        }

        // Get the skills value.
        let Some(skills) = character.get(SK2) else { return Err(Cow::from("Unable to find skills")) };
        if !skills.is_object() {
          return Err(Cow::from("Error reading skills"));
        }

        // Find a date.
        let Some(date) = find_date(skills) else { return Err(Cow::from("Unable to parse the date/time")) };

        Ok(GameData {
          path: RwLock::new(path),
          text,
          avatar,
          name,
          backpack,
          character,
          inventory,
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
    // Set CharacterSheet.
    let Some(text) = set_json(&self.text, "CharacterSheet", &self.avatar, &self.character) else { return Err(Cow::from("Unable to set CharacterSheet")) };

    // Set ItemStore.
    let Some(text) = set_json(&text, "ItemStore", &self.backpack, &self.inventory) else { return Err(Cow::from("Unable to set ItemStore")) };

    // Set UserGold.
    let Some(text) = set_json(&text, "UserGold", USER_ID, &self.gold) else { return Err(Cow::from("Unable to set UserGold")) };

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

  pub fn avatar_name(&self) -> &str {
    &self.name
  }

  pub fn get_gold(&self) -> Option<i32> {
    let gold = self.gold.get(G)?;
    let gold = gold.to_i64()?;
    Some(gold as i32)
  }

  pub fn set_gold(&mut self, gold: i32) {
    self.gold[G] = gold.into();
  }

  pub fn get_adv_lvl(&self) -> i32 {
    let exp = self.get_adv_exp();
    find_min(exp, &util::LEVEL_EXP).unwrap() as i32 + 1
  }

  pub fn set_adv_lvl(&mut self, lvl: i32) {
    assert!(util::LVL_RANGE.contains(&lvl));
    self.set_adv_exp(util::LEVEL_EXP[lvl as usize - 1]);
  }

  pub fn get_prd_lvl(&self) -> i32 {
    let exp = self.get_prd_exp();
    find_min(exp, &util::LEVEL_EXP).unwrap() as i32 + 1
  }

  pub fn set_prd_lvl(&mut self, lvl: i32) {
    assert!(util::LVL_RANGE.contains(&lvl));
    self.set_prd_exp(util::LEVEL_EXP[lvl as usize - 1]);
  }

  pub fn get_file_path(&self) -> PathBuf {
    self.path.read().unwrap().clone()
  }

  pub fn get_skills(&self, category: SkillCategory) -> Vec<SkillLvlGroup> {
    let sk2 = self.character.get(SK2).unwrap();
    let groups = util::parse_skill_group(category);
    let mut skills = Vec::with_capacity(groups.len());
    for group in groups {
      skills.push(SkillLvlGroup::new(sk2, group));
    }
    skills
  }

  pub fn set_skills(&mut self, skills: &Vec<SkillLvlGroup>) {
    let sk2 = self.character.get_mut(SK2).unwrap();
    for group in skills {
      for skill in &group.skills {
        set_skill_lvl(sk2, &self.date, skill);
      }
    }
  }

  pub fn get_inventory_items(&self) -> Vec<Item> {
    let items_val = self.inventory.get(IN).and_then(|v| v.as_object()).unwrap();
    let mut items = Vec::with_capacity(items_val.len());
    for (key, val) in items_val {
      let Some(val) = val.get(IN) else { continue };
      let Some(name) = get_item_name(val)  else { continue };
      let Some(cnt) = val.get(QN).and_then(|v| v.as_u64()) else { continue };
      let dur = Durability::new(val);
      let bag = val.get(BAG).is_some();

      items.push(Item {
        cnt_cmp: cnt,
        dur_cmp: dur.clone(),
        id: key.into(),
        name,
        cnt,
        dur,
        bag,
      });
    }
    items
  }

  pub fn set_inventory_items(&mut self, items: &Vec<Item>) {
    let items_val = self.inventory.get_mut(IN).unwrap();
    for item in items {
      let val = items_val.get_mut(&item.id).unwrap();
      let val = val.get_mut(IN).unwrap();
      val[QN] = item.cnt.into();
      if let Some(dur) = &item.dur {
        val[HP] = dur.minor.into();
        val[PHP] = dur.major.into();
      }
    }
  }

  fn get_adv_exp(&self) -> i64 {
    self.character.get(AE).unwrap().to_i64().unwrap()
  }

  fn set_adv_exp(&mut self, exp: i64) {
    self.character[AE] = exp.into();
  }

  fn get_prd_exp(&self) -> i64 {
    self.character.get(PE).unwrap().to_i64().unwrap()
  }

  fn set_prd_exp(&mut self, exp: i64) {
    self.character[PE] = exp.into();
  }
}

pub struct SkillLvl {
  info: SkillInfo,
  level: i32,
  comp: i32,
}

impl SkillLvl {
  fn new(sk2: &Value, info: SkillInfo) -> Self {
    let level = get_skill_lvl(sk2, &info).unwrap_or(0);
    let comp = level;
    Self { info, level, comp }
  }

  fn accept(&mut self) {
    self.comp = self.level;
  }

  fn discard(&mut self) {
    self.level = self.comp;
  }

  pub fn info(&self) -> &SkillInfo {
    &self.info
  }

  pub fn changed(&self) -> bool {
    self.level != self.comp
  }

  pub fn level(&self) -> i32 {
    self.level
  }

  pub fn level_mut(&mut self) -> &mut i32 {
    &mut self.level
  }
}

pub struct SkillLvlGroup {
  name: &'static str,
  skills: Vec<SkillLvl>,
}

impl SkillLvlGroup {
  fn new(sk2: &Value, group: SkillInfoGroup) -> Self {
    let name = group.name;
    let mut skills = Vec::with_capacity(group.skills.len());
    for skill in group.skills {
      skills.push(SkillLvl::new(sk2, skill));
    }
    Self { name, skills }
  }

  pub fn name(&self) -> &str {
    self.name
  }

  pub fn changed(&self) -> bool {
    for skill in &self.skills {
      if skill.changed() {
        return true;
      }
    }
    false
  }

  pub fn skills_mut(&mut self) -> &mut Vec<SkillLvl> {
    &mut self.skills
  }

  pub fn accept(&mut self) {
    for skill in self.skills_mut() {
      skill.accept();
    }
  }

  pub fn discard(&mut self) {
    for skill in self.skills_mut() {
      skill.discard();
    }
  }
}

#[derive(PartialEq, Clone)]
pub struct Durability {
  pub minor: f64,
  pub major: f64,
}

impl Durability {
  fn new(val: &Value) -> Option<Self> {
    let minor = val.get(HP)?.as_f64()?;
    let major = val.get(PHP)?.as_f64()?;
    Some(Durability { minor, major })
  }
}

#[derive(Clone)]
pub struct Item {
  cnt_cmp: u64,
  dur_cmp: Option<Durability>,
  id: String,
  name: String,
  cnt: u64,
  dur: Option<Durability>,
  bag: bool,
}

impl Item {
  pub fn changed(&self) -> bool {
    self.cnt != self.cnt_cmp || self.dur != self.dur_cmp
  }

  pub fn name(&self) -> &str {
    &self.name
  }

  pub fn count_mut(&mut self) -> &mut u64 {
    &mut self.cnt
  }

  pub fn durability_mut(&mut self) -> Option<&mut Durability> {
    if let Some(dur) = &mut self.dur {
      return Some(dur);
    }
    None
  }

  pub fn is_container(&self) -> bool {
    self.bag
  }

  pub fn accept(&mut self) {
    self.cnt = self.cnt_cmp;
    self.dur = self.dur_cmp.clone();
  }

  pub fn discard(&mut self) {
    self.cnt_cmp = self.cnt;
    self.dur_cmp = self.dur.clone();
  }
}

fn get_skill_lvl(sk2: &Value, info: &SkillInfo) -> Option<i32> {
  let exp = sk2.get(format!("{}", info.id))?.get(X)?;
  let exp = (exp.to_i64()? as f64 / info.mul) as i64;
  let idx = find_min(exp, &util::SKILL_EXP)?;
  Some(idx as i32 + 1)
}

fn set_skill_lvl(sk2: &mut Value, date: &Value, skill: &SkillLvl) {
  let lvl = skill.level();
  assert!((0..=200).contains(&lvl));
  if lvl == 0 {
    remove_skill(sk2, skill.info().id)
  } else {
    let exp = (util::SKILL_EXP[lvl as usize - 1] as f64 * skill.info().mul) as i64;
    let key = format!("{}", skill.info().id);
    if let Some(skill) = sk2.get_mut(&key) {
      skill[X] = exp.into();
    } else {
      sk2[key] = serde_json::json!({
        M: 0,
        T: date,
        X: exp,
      });
    }
  }
}

fn remove_skill(sk2: &mut Value, id: u64) {
  let skills = sk2.as_object_mut().unwrap();
  skills.remove(&format!("{}", id));
}

fn get_item_name(val: &Value) -> Option<String> {
  let text = val.get(AN)?.as_str()?;
  let pos = text.rfind('/')?;
  Some(text[pos + 1..].into())
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
const MAINBP: &str = "mainbp";
const BAG: &str = "bag";
const PHP: &str = "php";
const SK2: &str = "sk2";
const AE: &str = "ae";
const AN: &str = "an";
const DC: &str = "dc";
const FN: &str = "fn";
const HP: &str = "hp";
const IN: &str = "in";
const PE: &str = "pe";
const QN: &str = "qn";
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
  let json = get_json(text, "User", USER_ID)?;

  // Get the avatar ID.
  if let Some(Value::String(id)) = json.get(DC) {
    return Some(id.clone());
  }
  None
}

fn get_avatar_name(text: &str, avatar: &str) -> Option<String> {
  // Get the CharacterName json.
  let json = get_json(text, "CharacterName", avatar)?;

  // Get the avatar name.
  if let Some(Value::String(name)) = json.get(FN) {
    return Some(name.clone());
  }
  None
}

fn get_backpack_id(text: &str, avatar: &str) -> Option<String> {
  // Get the Character json.
  let json = get_json(text, "Character", avatar)?;

  // Get the backpack ID.
  if let Some(Value::String(id)) = json.get(MAINBP) {
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
  let mut result = String::with_capacity(parts[0].len() + parts[1].len() + parts[2].len());
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
