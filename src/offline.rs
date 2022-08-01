use self::game_info::GameInfo;
use crate::{game_data::GameData, util};
use eframe::{
  egui::{Button, DragValue, ImageButton, Response, RichText, Ui, WidgetText},
  emath,
  epaint::Color32,
};
use egui_extras::RetainedImage;
use std::{borrow::Cow, path::PathBuf};

pub struct Offline {
  load_image: RetainedImage,
  store_image: RetainedImage,
  game: Option<GameInfo>,
  error: Option<Cow<'static, str>>,
  modified: bool,
  load_request: bool,
}

impl Offline {
  pub fn new() -> Self {
    const LOAD_ICON: &[u8] = include_bytes!("res/load.png");
    const STORE_ICON: &[u8] = include_bytes!("res/store.png");

    let load_image = RetainedImage::from_image_bytes("load_image", LOAD_ICON).unwrap();
    let store_image = RetainedImage::from_image_bytes("store_image", STORE_ICON).unwrap();
    let game = None;
    let error = None;
    let modified = false;
    let load_request = false;

    Offline {
      load_image,
      store_image,
      game,
      error,
      modified,
      load_request,
    }
  }

  pub fn show(&mut self, ui: &mut Ui) {
    ui.horizontal(|ui| {
      if image_button(&self.load_image, ui)
        .on_hover_text_at_pointer("Load Save-game")
        .clicked()
      {
        self.load_request = true;
      }
      ui.add_enabled_ui(self.is_modified(), |ui| {
        if image_button(&self.store_image, ui)
          .on_hover_text_at_pointer("Store Save-game")
          .clicked()
        {
          self.store();
        }
      });

      ui.separator();

      ui.horizontal(|ui| {
        const LABEL_COLOR: Color32 = Color32::from_rgb(154, 187, 154);

        ui.label(RichText::from("Adventurer Level").color(LABEL_COLOR));
        if let Some(game) = &mut self.game {
          let mut level = game.adv_level();
          let widget = DragValue::new(&mut level).clamp_range(util::LVL_RANGE);
          if ui.add(widget).changed() {
            game.set_adv_level(level);
            self.modified = game.modified();
          }
        } else {
          ui.add_enabled_ui(false, |ui| {
            ui.add_sized(ui.spacing().interact_size, Button::new(RichText::default()));
          });
        }

        ui.label(RichText::from("Gold").color(LABEL_COLOR));
        if let Some(game) = &mut self.game {
          let mut gold = game.gold();
          let speed = (gold as f64 / 100.0).max(1.0);
          let range = 0..=i32::MAX;
          let widget = DragValue::new(&mut gold).speed(speed).clamp_range(range);
          if ui.add(widget).changed() {
            game.set_gold(gold);
            self.modified = game.modified();
          }
        } else {
          ui.add_enabled_ui(false, |ui| {
            ui.add_sized(ui.spacing().interact_size, Button::new(RichText::default()));
          });
        }
      });
    });

    ui.separator();

    if let Some(game) = &mut self.game {
      if game.show_skills(ui) {
        self.modified = game.modified();
      }
    }
  }

  pub fn show_status(&mut self, ui: &mut Ui) {
    ui.centered_and_justified(|ui| {
      if let Some(error) = &self.error {
        ui.label(WidgetText::from(error.as_ref()).color(Color32::LIGHT_RED));
      } else if let Some(file_name) = self.file_name() {
        ui.label(if self.is_modified() {
          format!("Editing {} (modified)", file_name)
        } else {
          format!("Editing {}", file_name)
        });
      }
    });
  }

  pub fn load(&mut self, path: PathBuf) {
    self.modified = false;
    match GameData::load(path) {
      Ok(game) => {
        self.game = Some(GameInfo::new(game));
        self.error = None;
      }
      Err(err) => {
        self.game = None;
        self.error = Some(err);
      }
    }
  }

  pub fn is_loaded(&self) -> bool {
    self.game.is_some()
  }

  pub fn is_modified(&self) -> bool {
    self.modified
  }

  pub fn store(&mut self) {
    let game = some!(&mut self.game);
    if let Err(err) = game.store() {
      self.error = Some(err);
    } else {
      self.modified = false;
    }
  }

  pub fn store_as(&mut self, path: PathBuf) {
    let game = some!(&mut self.game);
    if let Err(err) = game.store_as(path) {
      self.error = Some(err);
    } else {
      self.modified = false;
    }
  }

  pub fn discard(&mut self) {
    let game = some!(&mut self.game);
    game.discard_changes();
    self.modified = false;
  }

  pub fn file_name(&self) -> Option<String> {
    let game = self.game.as_ref()?;
    Some(game.get_file_name())
  }

  pub fn file_path(&self) -> Option<PathBuf> {
    let game = self.game.as_ref()?;
    Some(game.get_file_path())
  }

  pub fn load_request(&mut self) -> bool {
    let load_request = self.load_request;
    self.load_request = false;
    load_request
  }
}

fn image_button(image: &RetainedImage, ui: &mut Ui) -> Response {
  let texture_id = image.texture_id(ui.ctx());
  let image_size = emath::vec2(image.size()[0] as f32, image.size()[1] as f32);
  ui.add(ImageButton::new(texture_id, image_size))
}

mod game_info {
  use eframe::{
    egui::{CollapsingHeader, DragValue, Layout, RichText, ScrollArea, TextStyle, Ui},
    emath::{Align, Vec2},
    epaint::Color32,
  };
  use egui_extras::{Size, TableBuilder};

  use crate::{
    game_data::GameData,
    util::{self, Skill, SkillCategory, SkillGroup},
  };
  use std::{borrow::Cow, path::PathBuf};

  pub struct SkillLvl {
    info: Skill,
    level: i32,
    comp: i32,
  }

  impl SkillLvl {
    fn new(data: &GameData, info: Skill) -> Self {
      let level = data.get_skill_lvl(info.id, info.mul).unwrap_or(0);
      let comp = level;
      Self { info, level, comp }
    }
  }

  pub struct SkillLvlGroup {
    name: &'static str,
    skills: Vec<SkillLvl>,
  }

  impl SkillLvlGroup {
    fn new(data: &GameData, group: SkillGroup) -> Self {
      let name = group.name;
      let mut skills = Vec::with_capacity(group.skills.len());
      for skill in group.skills {
        skills.push(SkillLvl::new(data, skill));
      }
      Self { name, skills }
    }
  }

  pub struct GameInfo {
    data: GameData,
    adv: Vec<SkillLvlGroup>,
    prd: Vec<SkillLvlGroup>,
    level_cmp: i32,
    level: i32,
    gold_cmp: i32,
    gold: i32,
  }

  impl GameInfo {
    pub fn new(data: GameData) -> Self {
      let adv = {
        let groups = util::parse_skill_group(SkillCategory::Adventurer);
        let mut adv = Vec::with_capacity(groups.len());
        for group in groups {
          adv.push(SkillLvlGroup::new(&data, group));
        }
        adv
      };

      let prd = {
        let groups = util::parse_skill_group(SkillCategory::Producer);
        let mut prd = Vec::with_capacity(groups.len());
        for group in groups {
          prd.push(SkillLvlGroup::new(&data, group));
        }
        prd
      };

      let level = data.get_adv_lvl();
      let gold = data.get_gold().unwrap_or(0);

      GameInfo {
        data,
        adv,
        prd,
        level_cmp: level,
        level,
        gold_cmp: gold,
        gold,
      }
    }

    pub fn show_skills(&mut self, ui: &mut Ui) -> bool {
      // Divide the space evenly between adventurer and producer.
      let available = ui.available_size();
      let spacing = ui.spacing().item_spacing.y;
      let size = Vec2::new(available.x, available.y * 0.5 - spacing * 4.0);

      ui.add_space(spacing);

      // Adventurer skills.
      let mut changed = false;
      ui.allocate_ui(size, |ui| {
        if self.show_skill_category(ui, SkillCategory::Adventurer) {
          changed = true;
        }
      });

      ui.add_space(spacing);
      ui.separator();
      ui.add_space(spacing);

      // Producer skills.
      ui.allocate_ui(size, |ui| {
        if self.show_skill_category(ui, SkillCategory::Producer) {
          changed = true;
        }
      });

      changed
    }

    fn show_skill_category(&mut self, ui: &mut Ui, category: SkillCategory) -> bool {
      let row_size = TextStyle::Body.resolve(ui.style()).size + 4.0;
      let table_layout = Layout::left_to_right(Align::Center);
      let table_layout = table_layout.with_cross_align(Align::Center);
      let (scroll_id, groups) = match category {
        SkillCategory::Adventurer => ("offline_adventurer_skills", &mut self.adv),
        SkillCategory::Producer => ("offline_producer_skills", &mut self.prd),
      };

      let mut changed = false;
      ui.vertical(|ui| {
        ScrollArea::vertical()
          .id_source(scroll_id)
          .always_show_scroll(true)
          .show(ui, |ui| {
            for skill_group in groups {
              // Use a single column in order to force the scroll area to fill the entire available width.
              ui.columns(1, |col| {
                CollapsingHeader::new(skill_group.name)
                  .id_source(format!("{}_offline", skill_group.name.to_lowercase()))
                  .show(&mut col[0], |ui| {
                    TableBuilder::new(ui)
                      .cell_layout(table_layout)
                      .striped(true)
                      .scroll(false)
                      .column(Size::relative(0.64))
                      .column(Size::relative(0.18))
                      .column(Size::remainder())
                      .header(row_size, |mut header| {
                        const HEADER_COLOR: Color32 = Color32::from_rgb(229, 187, 123);
                        header.col(|ui| {
                          ui.label(RichText::from("Skill").color(HEADER_COLOR));
                        });
                        header.col(|ui| {
                          ui.label(RichText::from("Level").color(HEADER_COLOR));
                        });
                        header.col(|ui| {
                          ui.label(RichText::from("ID").color(HEADER_COLOR));
                        });
                      })
                      .body(|mut body| {
                        for skill in &mut skill_group.skills {
                          body.row(row_size, |mut row| {
                            row.col(|ui| {
                              let color = if skill.level > 0 {
                                const NAME_COLOR: Color32 = Color32::from_rgb(102, 154, 180);
                                NAME_COLOR
                              } else {
                                const SUBDUED_NAME_COLOR: Color32 = Color32::from_rgb(80, 120, 140);
                                SUBDUED_NAME_COLOR
                              };
                              ui.label(RichText::from(skill.info.name).color(color));
                            });
                            row.col(|ui| {
                              let widget = DragValue::new(&mut skill.level).clamp_range(0..=200);
                              if ui.add(widget).changed() {
                                changed = true;
                              }
                            });
                            row.col(|ui| {
                              ui.label(format!("{}", skill.info.id));
                            });
                          });
                        }
                      });
                  });
              });
            }
          });
      });

      changed
    }

    pub fn adv_level(&self) -> i32 {
      self.level
    }

    pub fn set_adv_level(&mut self, level: i32) {
      self.level = level
    }

    pub fn gold(&self) -> i32 {
      self.gold
    }

    pub fn set_gold(&mut self, gold: i32) {
      self.gold = gold;
    }

    pub fn store(&mut self) -> Result<(), Cow<'static, str>> {
      self.update_json();
      let result = self.data.store();
      if result.is_ok() {
        self.accept_changes();
      }
      result
    }

    pub fn store_as(&mut self, path: PathBuf) -> Result<(), Cow<'static, str>> {
      self.update_json();
      let result = self.data.store_as(path);
      if result.is_ok() {
        self.accept_changes();
      }
      result
    }

    pub fn modified(&self) -> bool {
      self.level != self.level_cmp
        || self.gold != self.gold_cmp
        || modified(&self.adv)
        || modified(&self.prd)
    }

    pub fn discard_changes(&mut self) {
      self.level = self.level_cmp;
      self.gold = self.gold_cmp;
      discard_changes(&mut self.adv);
      discard_changes(&mut self.prd);
    }

    pub fn get_file_path(&self) -> PathBuf {
      self.data.get_file_path()
    }

    pub fn get_file_name(&self) -> String {
      let path = self.get_file_path();
      path.file_name().unwrap().to_string_lossy().into()
    }

    fn accept_changes(&mut self) {
      self.level_cmp = self.level;
      self.gold_cmp = self.gold;
      accept_changes(&mut self.adv);
      accept_changes(&mut self.prd);
    }

    fn update_json(&mut self) {
      self.data.set_adv_lvl(self.level);
      self.data.set_gold(self.gold);
      update_json(&mut self.data, &self.adv);
      update_json(&mut self.data, &self.prd);
    }
  }

  fn modified(groups: &Vec<SkillLvlGroup>) -> bool {
    for group in groups {
      for skill in &group.skills {
        if skill.level != skill.comp {
          return true;
        }
      }
    }
    false
  }

  fn update_json(data: &mut GameData, groups: &Vec<SkillLvlGroup>) {
    for group in groups {
      for skill in &group.skills {
        data.set_skill_lvl(skill.info.id, skill.level, skill.info.mul);
      }
    }
  }

  fn accept_changes(groups: &mut Vec<SkillLvlGroup>) {
    for group in groups {
      for skill in &mut group.skills {
        skill.comp = skill.level;
      }
    }
  }

  fn discard_changes(groups: &mut Vec<SkillLvlGroup>) {
    for group in groups {
      for skill in &mut group.skills {
        skill.level = skill.comp;
      }
    }
  }
}
