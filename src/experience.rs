use crate::{
  config::Config,
  log_data,
  skill_info::{self, SkillCategory, SkillInfo, SkillInfoGroup},
  util::{self, AppState, Cancel, FAIL_ERR, LEVEL_EXP, NONE_ERR, SKILL_EXP},
};
use eframe::{
  egui::{
    scroll_area::ScrollBarVisibility, ComboBox, Context, DragValue, Label, Layout, RichText,
    ScrollArea, Sense, Ui, Widget,
  },
  emath::{Align, Vec2},
  epaint::Color32,
};
use egui_extras::{Column, TableBuilder};
use futures::{channel::mpsc, executor::ThreadPool};
use num_format::{Locale, ToFormattedString};
use std::{collections::HashMap, mem, path::PathBuf};

pub struct Experience {
  config: Config,
  state: AppState,
  threads: ThreadPool,
  channel: Channel,
  log_path: PathBuf,
  avatar: String,
  avatars: Vec<String>,
  adventurer_skills: Vec<SkillInfoGroup>,
  producer_skills: Vec<SkillInfoGroup>,
  level_info: LevelInfo,
  selected: SkillInfo,
  locale: Locale,
  init: bool,
}

impl Experience {
  pub fn new(
    log_path: PathBuf,
    threads: ThreadPool,
    config: Config,
    state: AppState,
    locale: Locale,
  ) -> Self {
    let (tx, rx) = mpsc::unbounded();
    let channel = Channel {
      tx,
      rx,
      cancel_avatars: None,
      cancel_adv_exp: None,
    };

    let adventurer_skills = skill_info::parse_skill_info_groups(SkillCategory::Adventurer);
    let producer_skills = skill_info::parse_skill_info_groups(SkillCategory::Producer);

    Experience {
      config,
      state,
      threads,
      channel,
      log_path,
      avatar: String::new(),
      avatars: Vec::new(),
      adventurer_skills,
      producer_skills,
      level_info: LevelInfo::new(),
      selected: Default::default(),
      locale,
      init: true,
    }
  }

  pub fn show(&mut self, ui: &mut Ui) {
    if mem::take(&mut self.init) {
      self.request_avatars(ui.ctx());
    }

    // Check for avatars.
    while let Ok(Some(msg)) = self.channel.rx.try_next() {
      self.state.set_busy(false);
      match msg {
        Message::Avatars(avatars) => {
          self.avatars = avatars;

          let mut avatar = self.avatar.clone();
          if avatar.is_empty() {
            // Get the avatar from the config file.
            if let Some(last_avatar) = self.config.get_exp_avatar() {
              avatar = last_avatar;
            }
          }

          if self.avatars.binary_search(&avatar).is_err() {
            avatar.clear();
          }

          if avatar.is_empty() {
            // Get the first avatar.
            if let Some(first) = self.avatars.first() {
              avatar = first.clone();
            }
          }

          self.set_avatar(avatar);
        }
        Message::AdvExp(exp) => {
          if let Some(exp) = exp {
            self.level_info.adv_exp = exp;
          }
        }
      }
    }

    // Tool bar.
    ui.horizontal(|ui| {
      ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
        ui.add_enabled_ui(!self.avatar.is_empty(), |ui| {
          // Adventurer level.
          let adv_info = self.get_adv_info();
          if let Some(adv_info) = &adv_info {
            let x_spacing = ui.spacing().item_spacing.x;
            ui.spacing_mut().item_spacing.x *= 0.5;

            let text = adv_info.exp.to_formatted_string(&self.locale);
            let response = Label::new(text).sense(Sense::click()).ui(ui);
            if response.on_hover_text("Click to copy").clicked() {
              util::set_clipboard_contents(format!("{}", adv_info.exp));
            }

            ui.spacing_mut().item_spacing.x = x_spacing;
            ui.label("Next");
          }

          let hover_text = "Type /xp in-game then click this button";
          let button_text = if let Some(adv_info) = &adv_info {
            format!("Adv Lvl {}", adv_info.lvl)
          } else {
            String::from("Adv Lvl ?")
          };
          if ui.button(button_text).on_hover_text(hover_text).clicked() {
            self.request_adv_exp(ui.ctx());
          }
        });
        ui.add_enabled_ui(!self.avatars.is_empty(), |ui| {
          // Avatar combo-box.
          let mut avatar_changed = None;
          ComboBox::from_id_source("exp_avatar_combo")
            .selected_text(&self.avatar)
            .width(ui.available_width())
            .show_ui(ui, |ui| {
              for avatar in &self.avatars {
                let response = ui.selectable_label(self.avatar == *avatar, avatar);
                if response.clicked() && self.avatar != *avatar {
                  avatar_changed = Some(avatar.clone());
                }
              }
            });

          if let Some(avatar) = avatar_changed {
            self.set_avatar(avatar)
          }
        });
      });
    });

    ui.separator();

    // Divide the remaining space evenly between adventurer and producer.
    let available = ui.available_size();
    let spacing = ui.spacing().item_spacing.y;
    let size = Vec2::new(available.x, available.y * 0.5 - spacing * 4.0);

    ui.add_space(spacing);

    // Adventurer skills.
    ui.allocate_ui(size, |ui| {
      self.show_skill_category(ui, SkillCategory::Adventurer);
    });

    ui.add_space(spacing);
    ui.separator();
    ui.add_space(spacing);

    // Producer skills.
    ui.allocate_ui(size, |ui| {
      self.show_skill_category(ui, SkillCategory::Producer);
    });
  }

  pub fn set_log_path(&mut self, ctx: &Context, log_path: PathBuf) {
    self.log_path = log_path;
    self.request_avatars(ctx);
  }

  fn show_skill_category(&mut self, ui: &mut Ui, category: SkillCategory) {
    let (scroll_id, groups) = match category {
      SkillCategory::Adventurer => ("adventurer_skills", &self.adventurer_skills),
      SkillCategory::Producer => ("producer_skills", &self.producer_skills),
    };

    ui.vertical(|ui| {
      ui.add_enabled_ui(!self.avatar.is_empty(), |ui| {
        ScrollArea::vertical()
          .id_source(scroll_id)
          .scroll_bar_visibility(ScrollBarVisibility::AlwaysVisible)
          .show(ui, |ui| {
            for skill_group in groups {
              // Use a single column in order to force the scroll area to fill the entire available width.
              ui.columns(1, |col| {
                let response = col[0].collapsing(skill_group.name, |ui| {
                  let spacing = ui.spacing().item_spacing;
                  let row_size = util::button_size(ui) + spacing[1];
                  TableBuilder::new(ui)
                    .cell_layout(Layout::left_to_right(Align::Center))
                    .striped(true)
                    .vscroll(false)
                    .column(Column::auto())
                    .column(Column::auto())
                    .column(Column::auto())
                    .column(Column::auto())
                    .column(Column::remainder())
                    .header(row_size, |mut header| {
                      const HEADER_COLOR: Color32 = Color32::from_rgb(229, 187, 123);
                      header.col(|ui| {
                        ui.label(RichText::from("Skill").color(HEADER_COLOR));
                      });
                      header.col(|ui| {
                        ui.label(RichText::from("Cur").color(HEADER_COLOR));
                      });
                      header.col(|ui| {
                        ui.label(RichText::from("Tgt").color(HEADER_COLOR));
                      });
                      header.col(|ui| {
                        ui.label(RichText::from("Mul").color(HEADER_COLOR));
                      });
                      header.col(|ui| {
                        ui.label(RichText::from("Exp").color(HEADER_COLOR));
                      });
                    })
                    .body(|mut body| {
                      for skill in &skill_group.skills {
                        let level = get_skill_lvl_mut(&mut self.level_info.skill_lvls, skill.id);
                        body.row(row_size, |mut row| {
                          row.col(|ui| {
                            let text = RichText::from(skill.name);
                            let text = text.color(Color32::from_rgb(102, 154, 180));
                            let widget = Label::new(text).wrap(false);
                            ui.add(widget);
                          });
                          row.col(|ui| {
                            let range = 0..=200;
                            let value = &mut level.0;
                            let widget = DragValue::new(value).clamp_range(range);
                            ui.add(widget);
                          });
                          row.col(|ui| {
                            let range = 0..=200;
                            let value = &mut level.1;
                            let widget = DragValue::new(value).clamp_range(range);
                            ui.add(widget);
                          });
                          row.col(|ui| {
                            ui.label(format!("{}x", skill.mul));
                          });
                          row.col(|ui| {
                            if let Some(exp) = get_needed_exp(level, skill.mul) {
                              let (text, exp) = if exp < 0 {
                                // Half experience returned for un-training.
                                let exp = exp / 2;
                                let text = exp.abs().to_formatted_string(&self.locale);
                                let text = format!("({})", text);
                                (text, exp)
                              } else {
                                let text = exp.to_formatted_string(&self.locale);
                                (text, exp)
                              };
                              let response = Label::new(text).sense(Sense::click()).ui(ui);
                              if response.on_hover_text("Click to copy").clicked() {
                                util::set_clipboard_contents(format!("{exp}"));
                              }
                            }
                          });
                        });
                      }
                    });
                });

                if response.header_response.clicked() {
                  // Check if this group contains the selected skill.
                  if skill_group
                    .skills
                    .binary_search_by(|skill| skill.name.cmp(self.selected.name))
                    .is_ok()
                  {
                    // Reset the selection.
                    self.selected = Default::default();
                  }
                }
              });
            }
          });
      });
    });
  }

  pub fn save(&mut self) {
    let avatar = &self.avatar;
    let skill_lvls = &self.level_info.skill_lvls;
    self.config.set_avatar_skills(avatar, skill_lvls);
  }

  pub fn on_exit(&mut self) {
    // Save the current values.
    self.save();

    // Cancel all async operations on exit.
    let cancelers = [
      self.channel.cancel_avatars.take(),
      self.channel.cancel_adv_exp.take(),
    ];

    for mut cancel in cancelers.into_iter().flatten() {
      cancel.cancel();
    }
  }

  fn request_avatars(&mut self, ctx: &Context) {
    // Cancel any previous request.
    if let Some(mut cancel) = self.channel.cancel_avatars.take() {
      cancel.cancel();
    }

    let cancel = Cancel::default();
    self.channel.cancel_avatars = Some(cancel.clone());

    // Show the busy cursor.
    self.state.set_busy(true);

    // Setup the future.
    let tx = self.channel.tx.clone();
    let ctx = ctx.clone();
    let future = log_data::get_avatars(self.log_path.clone(), cancel);
    let future = async move {
      let avatars = Message::Avatars(future.await);
      tx.unbounded_send(avatars).expect(FAIL_ERR);
      ctx.request_repaint();
    };

    // Execute the future on a pooled thread.
    self.threads.spawn_ok(future);
  }

  fn set_avatar(&mut self, avatar: String) {
    if self.avatar == avatar {
      return;
    }

    // Cancel any previous adventurer experience request.
    if let Some(mut cancel) = self.channel.cancel_adv_exp.take() {
      cancel.cancel();
    }

    // Save the current values.
    self.save();

    // Store the new avatar name.
    self.config.set_exp_avatar(avatar.clone());

    // Get the values for the new avatar.
    let skills = self
      .config
      .get_avatar_skills(&avatar)
      .unwrap_or(HashMap::new());

    self.level_info.skill_lvls = skills;
    self.level_info.adv_exp = 0;
    self.avatar = avatar;
  }

  fn request_adv_exp(&mut self, ctx: &Context) {
    if self.avatar.is_empty() {
      return;
    }

    // Cancel any previous request.
    if let Some(mut cancel) = self.channel.cancel_adv_exp.take() {
      cancel.cancel();
    }

    let cancel = Cancel::default();
    self.channel.cancel_adv_exp = Some(cancel.clone());

    // Show the busy cursor.
    self.state.set_busy(true);

    // Setup the future.
    let tx = self.channel.tx.clone();
    let ctx = ctx.clone();
    let future = log_data::get_adv_exp(self.log_path.clone(), self.avatar.clone(), cancel);
    let future = async move {
      let avatars = Message::AdvExp(future.await);
      tx.unbounded_send(avatars).expect(FAIL_ERR);
      ctx.request_repaint();
    };

    // Execute the future on a pooled thread.
    self.threads.spawn_ok(future);
  }

  fn get_adv_info(&self) -> Option<AdvInfo> {
    let exp = self.level_info.adv_exp;
    if exp > 0 {
      let lvl = util::find_min(exp, &LEVEL_EXP).expect(NONE_ERR) as i32 + 1;
      if lvl < 200 {
        return Some(AdvInfo {
          lvl,
          exp: LEVEL_EXP[lvl as usize] - exp,
        });
      } else {
        return Some(AdvInfo { lvl, exp: 0 });
      }
    }

    None
  }
}

struct AdvInfo {
  lvl: i32,
  exp: i64,
}

pub struct LevelInfo {
  pub adv_exp: i64,
  pub skill_lvls: HashMap<u32, (i32, i32)>,
}

impl LevelInfo {
  pub fn new() -> Self {
    LevelInfo {
      adv_exp: 0,
      skill_lvls: HashMap::new(),
    }
  }
}

enum Message {
  Avatars(Vec<String>),
  AdvExp(Option<i64>),
}

struct Channel {
  tx: mpsc::UnboundedSender<Message>,
  rx: mpsc::UnboundedReceiver<Message>,
  cancel_avatars: Option<Cancel>,
  cancel_adv_exp: Option<Cancel>,
}

fn get_skill_lvl_mut(levels: &mut HashMap<u32, (i32, i32)>, id: u32) -> &mut (i32, i32) {
  levels.entry(id).or_insert_with(|| (0, 0))
}

fn get_needed_exp(level: &(i32, i32), mul: f64) -> Option<i64> {
  if level.0 > 0 || level.1 > 0 {
    let cur_lvl = level.0.max(1);
    let tgt_lvl = level.1.max(1);
    let val = SKILL_EXP[tgt_lvl as usize - 1] - SKILL_EXP[cur_lvl as usize - 1];
    return Some((val as f64 * mul).ceil() as i64);
  }
  None
}
