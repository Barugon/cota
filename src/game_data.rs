use crate::{
  skill_info::{self, SkillCategory, SkillInfo, SkillInfoGroup},
  util::{FAIL_ERR, LEVEL_EXP, LVL_RANGE, NONE_ERR, SKILL_EXP},
};
use serde_json::Value;
use std::{borrow::Cow, fs::File, io::Write, ops::Range, path::PathBuf, sync::RwLock};

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
        let avatar = get_avatar_id(&text)?;

        // Get the avatar name.
        let name = get_avatar_name(&text, &avatar)?;

        // Get the backpack ID.
        let backpack = get_backpack_id(&text, &avatar)?;

        // Get the ItemStore JSON.
        let inventory = get_json(&text, ITEM_STORE, &backpack)?;

        // Get the CharacterSheet JSON.
        let character = get_json(&text, CHARACTER_SHEET, &avatar)?;

        // Make sure adventurer experience is there.
        if character.get(AE).and_then(|exp| exp.to_i64()).is_none() {
          return Err(Cow::from("Unable to parse adventurer experience"));
        }

        // Make sure producer experience is there.
        if character.get(PE).and_then(|exp| exp.to_i64()).is_none() {
          return Err(Cow::from("Unable to parse producer experience"));
        }

        // Find a save date.
        let date = match character.get(SK2) {
          Some(val) if val.is_object() => find_date(val)?,
          _ => return Err(Cow::from("Error reading skills")),
        };

        // Get the UserGold JSON.
        let gold = get_json(&text, USER_GOLD, USER_ID)?;

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
      Err(err) => Err(Cow::from(format!("Unable to load file: {err}"))),
    }
  }

  pub fn store(&self) -> Result<(), Cow<'static, str>> {
    self.store_as(self.get_file_path())
  }

  pub fn store_as(&self, path: PathBuf) -> Result<(), Cow<'static, str>> {
    // Set CharacterSheet.
    let text = set_json(&self.text, CHARACTER_SHEET, &self.avatar, &self.character)?;

    // Set ItemStore.
    let text = set_json(&text, ITEM_STORE, &self.backpack, &self.inventory)?;

    // Set UserGold.
    let text = set_json(&text, USER_GOLD, USER_ID, &self.gold)?;

    // Create the save-game file and store the data.
    match File::create(&path) {
      Ok(mut file) => match file.write_all(text.as_bytes()) {
        Ok(()) => {
          // Change the path.
          *self.path.write().expect(FAIL_ERR) = path;
          Ok(())
        }
        Err(err) => Err(Cow::from(err.to_string())),
      },
      Err(err) => Err(Cow::from(err.to_string())),
    }
  }

  pub fn avatar_name(&self) -> &str {
    &self.name
  }

  pub fn get_gold(&self) -> Option<i32> {
    Some(self.gold.get(G)?.to_i64()? as i32)
  }

  pub fn set_gold(&mut self, gold: i32) {
    self.gold[G] = gold.into();
  }

  pub fn get_adv_lvl(&self) -> i32 {
    let ae = self.character.get(AE).expect(NONE_ERR);
    let exp = ae.to_i64().expect(NONE_ERR);
    find_min(exp, &LEVEL_EXP).expect(NONE_ERR) as i32 + 1
  }

  pub fn set_adv_lvl(&mut self, lvl: i32) {
    assert!(LVL_RANGE.contains(&lvl));
    self.character[AE] = LEVEL_EXP[lvl as usize - 1].into();
  }

  pub fn get_prd_lvl(&self) -> i32 {
    let pe = self.character.get(PE).expect(NONE_ERR);
    let exp = pe.to_i64().expect(NONE_ERR);
    find_min(exp, &LEVEL_EXP).expect(NONE_ERR) as i32 + 1
  }

  pub fn set_prd_lvl(&mut self, lvl: i32) {
    assert!(LVL_RANGE.contains(&lvl));
    self.character[PE] = LEVEL_EXP[lvl as usize - 1].into();
  }

  pub fn get_file_path(&self) -> PathBuf {
    self.path.read().expect(FAIL_ERR).clone()
  }

  pub fn get_file_name(&self) -> String {
    let path = self.path.read().expect(FAIL_ERR);
    path.file_name().expect(NONE_ERR).to_string_lossy().into()
  }

  pub fn get_skills(&self, category: SkillCategory) -> Vec<SkillLvlGroup> {
    let sk2 = self.character.get(SK2).expect(NONE_ERR);
    let groups = skill_info::parse_skill_info_groups(category);
    let mut skills = Vec::with_capacity(groups.len());
    for group in groups {
      skills.push(SkillLvlGroup::new(sk2, group));
    }

    skills
  }

  pub fn set_skills(&mut self, skills: &Vec<SkillLvlGroup>) {
    let sk2 = self.character.get_mut(SK2).expect(NONE_ERR);
    for group in skills {
      for skill in &group.skills {
        set_skill_lvl(sk2, &self.date, skill);
      }
    }
  }

  pub fn get_inventory_items(&self) -> Vec<Item> {
    let inv = self.inventory.get(IN).expect(NONE_ERR);
    let items_map = inv.as_object().expect(NONE_ERR);
    let mut items = Vec::with_capacity(items_map.len());
    for (key, val) in items_map {
      if let Some(item) = Item::new(val, key) {
        items.push(item);
      }
    }

    items
  }

  pub fn set_inventory_items(&mut self, items: &Vec<Item>) {
    let inv = self.inventory.get_mut(IN).expect(NONE_ERR);
    for item in items {
      let val = inv.get_mut(&item.id).expect(NONE_ERR);
      let val = val.get_mut(IN).expect(NONE_ERR);
      val[QN] = item.cnt.into();
      if let Some(dur) = &item.dur {
        val[HP] = dur.minor.into();
        val[PHP] = dur.major.into();
      }
    }
  }
}

const USER_ID: &str = "000000000000000000000001";
const CHARACTER_SHEET: &str = "CharacterSheet";
const ITEM_STORE: &str = "ItemStore";
const USER_GOLD: &str = "UserGold";
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

#[derive(Clone)]
pub struct SkillLvl {
  pub info: SkillInfo,
  pub level: i32,
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

  pub fn changed(&self) -> bool {
    self.level != self.comp
  }
}

pub struct SkillLvlGroup {
  pub name: &'static str,
  pub skills: Vec<SkillLvl>,
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

  pub fn changed(&self) -> bool {
    for skill in &self.skills {
      if skill.changed() {
        return true;
      }
    }

    false
  }

  pub fn accept(&mut self) {
    for skill in &mut self.skills {
      skill.accept();
    }
  }

  pub fn discard(&mut self) {
    for skill in &mut self.skills {
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
  id: String,
  name: String,
  cnt_cmp: u64,
  cnt: u64,
  dur_cmp: Option<Durability>,
  dur: Option<Durability>,
  bag: bool,
}

impl Item {
  fn new(val: &Value, id: &str) -> Option<Self> {
    let val = val.get(IN)?;
    let name = get_item_name(val)?;
    let cnt = val.get(QN).and_then(|v| v.as_u64())?;
    let dur = Durability::new(val);
    let bag = val.get(BAG).is_some();

    Some(Item {
      id: id.into(),
      name,
      cnt_cmp: cnt,
      cnt,
      dur_cmp: dur.clone(),
      dur,
      bag,
    })
  }

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
    self.cnt_cmp = self.cnt;
    self.dur_cmp = self.dur.clone();
  }

  pub fn discard(&mut self) {
    self.cnt = self.cnt_cmp;
    self.dur = self.dur_cmp.clone();
  }
}

fn get_skill_lvl(sk2: &Value, info: &SkillInfo) -> Option<i32> {
  let exp = sk2.get(format!("{}", info.id))?.get(X)?;
  let exp = (exp.to_i64()? as f64 / info.mul) as i64;
  let idx = find_min(exp, &SKILL_EXP)?;

  Some(idx as i32 + 1)
}

fn set_skill_lvl(sk2: &mut Value, date: &Value, skill: &SkillLvl) {
  assert!((0..=200).contains(&skill.level));
  if skill.level == 0 {
    remove_skill(sk2, skill.info.id)
  } else {
    let exp = (SKILL_EXP[skill.level as usize - 1] as f64 * skill.info.mul).ceil() as i64;
    let key = format!("{}", skill.info.id);
    if let Some(skill) = sk2.get_mut(&key) {
      // Set the skill's experience.
      skill[X] = exp.into();
    } else {
      // Skill doesn't exist, so add it.
      sk2[key] = serde_json::json!({
        M: 0,
        T: date,
        X: exp,
      });
    }
  }
}

fn remove_skill(sk2: &mut Value, id: u32) {
  let skills = sk2.as_object_mut().expect(NONE_ERR);
  skills.remove(&format!("{id}"));
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

fn get_avatar_id(text: &str) -> Result<String, Cow<'static, str>> {
  // Get the User json.
  let json = get_json(text, "User", USER_ID)?;

  // Get the avatar ID.
  if let Some(Value::String(id)) = json.get(DC) {
    return Ok(id.clone());
  }

  Err(Cow::from("Unable to determine the current avatar"))
}

fn get_avatar_name(text: &str, avatar: &str) -> Result<String, Cow<'static, str>> {
  // Get the CharacterName json.
  let json = get_json(text, "CharacterName", avatar)?;

  // Get the avatar name.
  if let Some(Value::String(name)) = json.get(FN) {
    return Ok(name.clone());
  }

  Err(Cow::from("Unable to get the avatar name"))
}

fn get_backpack_id(text: &str, avatar: &str) -> Result<String, Cow<'static, str>> {
  // Get the Character json.
  let json = get_json(text, "Character", avatar)?;

  // Get the backpack ID.
  if let Some(Value::String(id)) = json.get("mainbp") {
    return Ok(id.clone());
  }

  Err(Cow::from("Unable to find the avatar's backpack"))
}

fn collection_tag(collection: &str) -> String {
  format!(r#"<collection name="{collection}">"#)
}

fn record_tag(id: &str) -> String {
  format!(r#"<record Id="{id}">"#)
}

const fn record_end() -> &'static str {
  "</record>"
}

fn get_json_range(text: &str, collection: &str, id: &str) -> Option<Range<usize>> {
  // Find the collection tag.
  let find = collection_tag(collection);
  let start = text.find(&find)? + find.len();
  let text = &text[start..];

  // From that point, find the record tag.
  let find = record_tag(id);
  let pos = text.find(&find)? + find.len();
  let text = &text[pos..];
  let start = start + pos;

  // Find the record end tag.
  let pos = text.find(record_end())?;
  let end = start + pos;

  Some(start..end)
}

fn get_json(text: &str, collection: &str, id: &str) -> Result<Value, Cow<'static, str>> {
  if let Some(range) = get_json_range(text, collection, id) {
    let text = &text[range];
    match serde_json::from_str::<Value>(text) {
      Ok(val) if val.is_object() => return Ok(val),
      Err(err) => return Err(Cow::from(err.to_string())),
      _ => (),
    }
  }

  let err = format!("Unable to get '{collection}' collection");
  Err(Cow::from(err))
}

fn set_json(
  text: &str,
  collection: &str,
  id: &str,
  val: &Value,
) -> Result<String, Cow<'static, str>> {
  if let Some(range) = get_json_range(text, collection, id) {
    // Convert the value to JSON text.
    let json = val.to_string();

    // Concatenate the XML with the new JSON.
    let parts = [&text[..range.start], &json, &text[range.end..]];
    let mut result = String::with_capacity(parts[0].len() + parts[1].len() + parts[2].len());
    result.push_str(parts[0]);
    result.push_str(parts[1]);
    result.push_str(parts[2]);
    return Ok(result);
  }

  let err = format!("Unable to set '{collection}' collection");
  Err(Cow::from(err))
}

fn find_date(val: &Value) -> Result<Value, Cow<'static, str>> {
  if let Value::Object(obj) = val {
    for (_, val) in obj {
      if let Some(val) = val.get(T) {
        return Ok(val.clone());
      }
    }
  }

  Err(Cow::from("Unable to find a save date"))
}
