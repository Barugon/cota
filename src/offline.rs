use self::game_info::GameInfo;
use crate::{
  game_data::GameData,
  items_dlg::ItemsDlg,
  util::{self, AppState},
};
use eframe::{
  egui::{Button, DragValue, ImageButton, Response, RichText, Ui, WidgetText},
  emath,
  epaint::Color32,
};
use egui_extras::RetainedImage;
use std::{borrow::Cow, path::PathBuf};

pub struct Offline {
  items_dlg: ItemsDlg,
  load_image: RetainedImage,
  store_image: RetainedImage,
  game: Option<GameInfo>,
  error: Option<Cow<'static, str>>,
  changed: bool,
  load_request: bool,
}

impl Offline {
  pub fn new(state: AppState) -> Self {
    const LOAD_ICON: &[u8] = include_bytes!("res/load.png");
    const STORE_ICON: &[u8] = include_bytes!("res/store.png");

    let load_image = RetainedImage::from_image_bytes("load_image", LOAD_ICON).unwrap();
    let store_image = RetainedImage::from_image_bytes("store_image", STORE_ICON).unwrap();
    let game = None;
    let error = None;
    let changed = false;
    let load_request = false;

    Offline {
      items_dlg: ItemsDlg::new(state),
      load_image,
      store_image,
      game,
      error,
      changed,
      load_request,
    }
  }

  pub fn show(&mut self, ui: &mut Ui) {
    if let Some(game) = &mut self.game {
      if self.items_dlg.show(game.items_mut(), ui.ctx()) {
        self.changed = game.changed();
      }
    }

    // Tool bar.
    ui.horizontal(|ui| {
      if image_button(&self.load_image, ui)
        .on_hover_text_at_pointer("Load Save-game")
        .clicked()
      {
        self.load_request = true;
      }
      ui.add_enabled_ui(self.changed(), |ui| {
        if image_button(&self.store_image, ui)
          .on_hover_text_at_pointer("Store Save-game")
          .clicked()
        {
          self.store();
        }
      });

      ui.separator();

      ui.horizontal(|ui| {
        ui.add_enabled_ui(self.game.is_some(), |ui| {
          if ui.button("Items").clicked() {
            self.items_dlg.open();
          }
        });
      });

      ui.separator();

      ui.horizontal(|ui| {
        const LABEL_COLOR: Color32 = Color32::from_rgb(154, 187, 154);

        ui.label(RichText::from("Adv Lvl").color(LABEL_COLOR));
        if let Some(game) = &mut self.game {
          let mut level = game.adv_level();
          let widget = DragValue::new(&mut level).clamp_range(util::LVL_RANGE);
          if ui.add(widget).changed() {
            game.set_adv_level(level);
            self.changed = game.changed();
          }
        } else {
          ui.add_enabled_ui(false, |ui| {
            ui.add_sized(ui.spacing().interact_size, Button::new(RichText::default()));
          });
        }

        ui.label(RichText::from("Prd Lvl").color(LABEL_COLOR));
        if let Some(game) = &mut self.game {
          let mut level = game.prd_level();
          let widget = DragValue::new(&mut level).clamp_range(util::LVL_RANGE);
          if ui.add(widget).changed() {
            game.set_prd_level(level);
            self.changed = game.changed();
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
          let range = 0..=MAX_GOLD;
          let widget = DragValue::new(&mut gold).speed(speed).clamp_range(range);
          if ui.add(widget).changed() {
            game.set_gold(gold);
            self.changed = game.changed();
          }
        } else {
          ui.add_enabled_ui(false, |ui| {
            ui.add_sized(ui.spacing().interact_size, Button::new(RichText::default()));
          });
        }
      });
    });

    ui.separator();

    // Skills.
    if let Some(game) = &mut self.game {
      if game.show_skills(ui) {
        self.changed = game.changed();
      }
    }
  }

  pub fn show_status(&mut self, ui: &mut Ui) {
    ui.centered_and_justified(|ui| {
      if let Some(error) = &self.error {
        ui.label(WidgetText::from(error.as_ref()).color(Color32::LIGHT_RED));
      } else if let Some(game) = self.game.as_ref() {
        let file_name = game.get_file_name();
        let changed = if self.changed() { "*" } else { "" };
        ui.label(format!(
          "Editing {} - {}{}",
          game.avatar_name(),
          changed,
          file_name
        ));
      }
    });
  }

  pub fn load(&mut self, path: PathBuf) -> bool {
    self.changed = false;
    match GameData::load(path) {
      Ok(game) => {
        self.game = Some(GameInfo::new(game));
        self.error = None;
        true
      }
      Err(err) => {
        self.game = None;
        self.error = Some(err);
        false
      }
    }
  }

  pub fn is_loaded(&self) -> bool {
    self.game.is_some()
  }

  pub fn changed(&self) -> bool {
    self.changed
  }

  pub fn store(&mut self) {
    let Some(game) = &mut self.game else { return };
    if let Err(err) = game.store() {
      self.error = Some(err);
    } else {
      self.changed = false;
    }
  }

  pub fn store_as(&mut self, path: PathBuf) {
    let Some(game) = &mut self.game else { return };
    if let Err(err) = game.store_as(path) {
      self.error = Some(err);
    } else {
      self.changed = false;
    }
  }

  pub fn discard(&mut self) {
    let Some(game) = &mut self.game else { return };
    game.discard_changes();
    self.changed = false;
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

  pub fn on_close_event(&mut self) {
    self.items_dlg.close();
  }
}

fn image_button(image: &RetainedImage, ui: &mut Ui) -> Response {
  let texture_id = image.texture_id(ui.ctx());
  let image_size = emath::vec2(image.size()[0] as f32, image.size()[1] as f32);
  ui.add(ImageButton::new(texture_id, image_size))
}

const MAX_GOLD: i32 = i32::MAX / 2;

mod game_info {
  use crate::{
    game_data::{GameData, Item, SkillLvlGroup},
    util::{self, SkillCategory},
  };
  use eframe::{
    egui::{CollapsingHeader, DragValue, Layout, RichText, ScrollArea, Ui},
    emath::{Align, Vec2},
    epaint::Color32,
  };
  use egui_extras::{Column, TableBuilder};
  use std::{borrow::Cow, ffi::OsStr, path::PathBuf};

  pub struct GameInfo {
    data: GameData,
    adv: Vec<SkillLvlGroup>,
    prd: Vec<SkillLvlGroup>,
    items: Vec<Item>,
    adv_lvl_cmp: i32,
    adv_lvl: i32,
    prd_lvl_cmp: i32,
    prd_lvl: i32,
    gold_cmp: i32,
    gold: i32,
  }

  impl GameInfo {
    pub fn new(data: GameData) -> Self {
      let adv = data.get_skills(SkillCategory::Adventurer);
      let prd = data.get_skills(SkillCategory::Producer);
      let items = data.get_inventory_items();
      let adv_lvl = data.get_adv_lvl();
      let prd_lvl = data.get_prd_lvl();
      let gold = data.get_gold().unwrap_or(0);

      GameInfo {
        data,
        adv,
        prd,
        items,
        adv_lvl_cmp: adv_lvl,
        adv_lvl,
        prd_lvl_cmp: prd_lvl,
        prd_lvl,
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
                CollapsingHeader::new(skill_group.name())
                  .id_source(format!("{}_offline", skill_group.name().to_lowercase()))
                  .show(&mut col[0], |ui| {
                    let spacing = ui.spacing().item_spacing;
                    let row_size = util::button_size(ui) + spacing[1] * 2.0;
                    let available_width = ui.available_width();
                    TableBuilder::new(ui)
                      .cell_layout(Layout::left_to_right(Align::Center))
                      .striped(true)
                      .vscroll(false)
                      .column(Column::exact(available_width * 0.64 - spacing[0]))
                      .column(Column::exact(available_width * 0.18 - spacing[0]))
                      .column(Column::remainder())
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
                        for skill in skill_group.skills_mut() {
                          body.row(row_size, |mut row| {
                            row.col(|ui| {
                              let color = if skill.level() > 0 {
                                const NAME_COLOR: Color32 = Color32::from_rgb(102, 154, 180);
                                NAME_COLOR
                              } else {
                                const SUBDUED_NAME_COLOR: Color32 = Color32::from_rgb(80, 120, 140);
                                SUBDUED_NAME_COLOR
                              };
                              ui.label(RichText::from(skill.info().name).color(color));
                            });
                            row.col(|ui| {
                              let widget = DragValue::new(skill.level_mut()).clamp_range(0..=200);
                              if ui.add(widget).changed() {
                                changed = true;
                              }
                            });
                            row.col(|ui| {
                              ui.label(format!("{}", skill.info().id));
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

    pub fn avatar_name(&self) -> &str {
      self.data.avatar_name()
    }

    pub fn items_mut(&mut self) -> &mut Vec<Item> {
      &mut self.items
    }

    pub fn adv_level(&self) -> i32 {
      self.adv_lvl
    }

    pub fn set_adv_level(&mut self, level: i32) {
      self.adv_lvl = level
    }

    pub fn prd_level(&self) -> i32 {
      self.prd_lvl
    }

    pub fn set_prd_level(&mut self, level: i32) {
      self.prd_lvl = level
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
      // Make sure the extension is "sota".
      let path = if path.extension() != Some(OsStr::new("sota")) {
        path.with_extension("sota")
      } else {
        path
      };

      self.update_json();
      let result = self.data.store_as(path);
      if result.is_ok() {
        self.accept_changes();
      }
      result
    }

    pub fn changed(&self) -> bool {
      self.adv_lvl != self.adv_lvl_cmp
        || self.prd_lvl != self.prd_lvl_cmp
        || self.gold_changed()
        || self.items_changed()
        || changed(&self.adv)
        || changed(&self.prd)
    }

    pub fn discard_changes(&mut self) {
      for item in &mut self.items {
        item.discard();
      }
      self.adv_lvl = self.adv_lvl_cmp;
      self.prd_lvl = self.prd_lvl_cmp;
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
      // Since gold can be larger than the editor maximum, we need to check here.
      if self.gold_changed() {
        self.gold_cmp = self.gold;
      }

      for item in &mut self.items {
        item.accept();
      }
      self.adv_lvl_cmp = self.adv_lvl;
      self.prd_lvl_cmp = self.prd_lvl;
      accept_changes(&mut self.adv);
      accept_changes(&mut self.prd);
    }

    fn update_json(&mut self) {
      self.data.set_inventory_items(&self.items);
      self.data.set_adv_lvl(self.adv_lvl);
      self.data.set_prd_lvl(self.prd_lvl);
      self.data.set_gold(self.gold);
      self.data.set_skills(&self.adv);
      self.data.set_skills(&self.prd);
    }

    fn gold_changed(&self) -> bool {
      if self.gold != self.gold_cmp {
        return self.gold_cmp < self.gold || self.gold != super::MAX_GOLD;
      }
      false
    }

    fn items_changed(&self) -> bool {
      for item in &self.items {
        if item.changed() {
          return true;
        }
      }
      false
    }
  }

  fn changed(groups: &Vec<SkillLvlGroup>) -> bool {
    for group in groups {
      if group.changed() {
        return true;
      }
    }
    false
  }

  fn accept_changes(groups: &mut Vec<SkillLvlGroup>) {
    for group in groups {
      group.accept();
    }
  }

  fn discard_changes(groups: &mut Vec<SkillLvlGroup>) {
    for group in groups {
      group.discard();
    }
  }
}
