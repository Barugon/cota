#[derive(Clone, Copy, Debug)]
pub enum SkillCategory {
  Adventurer,
  Producer,
}

#[derive(Clone, Default, serde::Deserialize, serde::Serialize)]
pub struct Requires {
  pub id: u32,
  pub lvl: i32,
}

#[derive(Clone, Default, serde::Deserialize, serde::Serialize)]
pub struct SkillInfo {
  pub name: &'static str,
  pub mul: f64,
  pub id: u32,
  pub reqs: Vec<Requires>,
}

#[derive(serde::Deserialize, serde::Serialize)]
pub struct SkillInfoGroup {
  pub name: &'static str,
  pub skills: Vec<SkillInfo>,
}

/// Parse the CSV for adventurer or producer skills.
pub fn parse_skill_info_groups(category: SkillCategory) -> Vec<SkillInfoGroup> {
  serde_json::from_str(match category {
    SkillCategory::Adventurer => include_str!("../res/adventurer_skills.json"),
    SkillCategory::Producer => include_str!("../res/producer_skills.json"),
  })
  .unwrap()
}

#[allow(unused)]
pub fn write_skill_info_groups<P: AsRef<std::path::Path>>(path: P, skill_groups: &[SkillInfoGroup]) {
  if let Ok(text) = serde_json::to_string(skill_groups) {
    std::fs::write(path, text).ok();
  }
}
