use serde::{Deserialize, Serialize};

use crate::util::{FAIL_ERR, NONE_ERR};

#[derive(Default, Serialize, Deserialize)]
pub struct SkillLvlPlan {
  pub cur: i32,
  pub tgt: i32,
}

#[derive(Clone, Copy, Debug)]
pub enum SkillCategory {
  Adventurer,
  Producer,
}

#[derive(Clone, Default)]
pub struct Requires {
  pub id: u32,
  pub lvl: i32,
}

#[derive(Clone, Default)]
pub struct SkillInfo {
  pub name: &'static str,
  pub mul: f64,
  pub id: u32,
  pub reqs: Vec<Requires>,
}

#[derive(Default)]
pub struct SkillInfoGroup {
  pub name: &'static str,
  pub skills: Vec<SkillInfo>,
}

impl SkillInfoGroup {
  fn new(name: &'static str) -> Self {
    Self {
      name,
      skills: Vec::new(),
    }
  }
}

/// Parse the CSV for adventurer or producer skills.
pub fn parse_skill_info_groups(category: SkillCategory) -> Vec<SkillInfoGroup> {
  let text = match category {
    SkillCategory::Adventurer => include_str!("res/adventurer_skills.csv"),
    SkillCategory::Producer => include_str!("res/producer_skills.csv"),
  };
  let mut skill_groups = Vec::new();
  let mut skill_group = SkillInfoGroup::default();

  // Temporary vector to hold skill requirements in order to keep memory use and reallocations low.
  let mut tmp_reqs = Vec::new();

  for line in text.lines() {
    let mut fields = line.split(',');
    if let Some(group) = fields.next() {
      // CSVs are pre-sorted, so we just need to create a new group on group change.
      if group != skill_group.name {
        if !skill_group.name.is_empty() {
          skill_groups.push(skill_group);
        }
        skill_group = SkillInfoGroup::new(group);
      }

      let name = fields.next().expect(NONE_ERR);
      let mul = fields.next().expect(NONE_ERR).parse().expect(FAIL_ERR);
      let id = fields.next().expect(NONE_ERR).parse().expect(FAIL_ERR);

      while let Some(id) = fields.next() {
        let id = id.parse().expect(FAIL_ERR);
        let lvl = fields.next().expect(NONE_ERR).parse().expect(FAIL_ERR);
        tmp_reqs.push(Requires { id, lvl });
      }

      let reqs = tmp_reqs.clone();
      tmp_reqs.clear();

      skill_group.skills.push(SkillInfo {
        name,
        mul,
        id,
        reqs,
      });
    }
  }

  if !skill_group.name.is_empty() {
    skill_groups.push(skill_group);
  }

  skill_groups
}
