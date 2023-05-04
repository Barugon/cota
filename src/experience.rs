use crate::{
  config, log_data,
  skill_info::{self, AvatarPlan, SkillCategory, SkillInfo, SkillInfoGroup},
  util::{self, AppState, Cancel, FAIL_ERR, LEVEL_EXP, LVL_RANGE, NONE_ERR, SKILL_EXP},
};
use eframe::{
  egui::{ComboBox, Context, DragValue, Label, Layout, RichText, ScrollArea, Sense, Ui, Widget},
  emath::{Align, Vec2},
  epaint::Color32,
  Storage,
};
use egui_extras::{Column, TableBuilder};
use futures::{channel::mpsc, executor::ThreadPool};
use num_format::{Locale, ToFormattedString};
use std::{collections::HashMap, mem, path::PathBuf, sync::Mutex};

pub struct Experience {
  state: AppState,
  threads: ThreadPool,
  channel: Channel,
  log_path: PathBuf,
  avatar: String,
  avatars: Vec<String>,
  adventurer_skills: Vec<SkillInfoGroup>,
  producer_skills: Vec<SkillInfoGroup>,
  avatar_plan: Mutex<AvatarPlan>,
  selected: SkillInfo,
  locale: Locale,
  init: bool,
}

impl Experience {
  pub fn new(log_path: PathBuf, threads: ThreadPool, state: AppState, locale: Locale) -> Self {
    let (tx, rx) = mpsc::unbounded();
    let channel = Channel {
      tx,
      rx,
      cancel_avatars: None,
    };

    let adventurer_skills = skill_info::parse_skill_info_groups(SkillCategory::Adventurer);
    let producer_skills = skill_info::parse_skill_info_groups(SkillCategory::Producer);

    Experience {
      state,
      threads,
      channel,
      log_path,
      avatar: String::new(),
      avatars: Vec::new(),
      adventurer_skills,
      producer_skills,
      avatar_plan: Mutex::new(AvatarPlan::new()),
      selected: Default::default(),
      locale,
      init: true,
    }
  }

  pub fn show(&mut self, ui: &mut Ui, frame: &mut eframe::Frame) {
    if mem::take(&mut self.init) {
      self.request_avatars(ui.ctx());
    }

    // Check for avatars.
    while let Ok(Some(avatars)) = self.channel.rx.try_next() {
      self.state.set_busy(false);
      self.avatars = avatars;

      let mut avatar = self.avatar.clone();
      if avatar.is_empty() {
        // Get the avatar from the config file.
        if let Some(last_avatar) = config::get_exp_avatar(frame.storage().expect(NONE_ERR)) {
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

      self.set_avatar(frame.storage_mut().expect(NONE_ERR), avatar);
    }

    // Tool bar.
    ui.horizontal(|ui| {
      ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
        // Adventurer level.
        ui.scope(|ui| {
          const LABEL_COLOR: Color32 = Color32::from_rgb(154, 187, 154);
          let x_spacing = ui.spacing().item_spacing.x;
          let mut plan = self.avatar_plan.lock().expect(FAIL_ERR);
          let value = &mut plan.adv_lvl;

          // Spacing adjustment occurs to the left of experience since the layout here is right to left.
          ui.spacing_mut().item_spacing.x *= 0.5;

          // This allocates enough space for the maximum experience value size and uses left to right
          // layout internally to keep the experience label left justified.
          ui.allocate_ui_with_layout(
            Vec2::new(107.0, ui.available_height()),
            Layout::left_to_right(Align::Center),
            |ui| {
              let value = *value;
              let next = (value + 1).min(200);
              let exp = LEVEL_EXP[next as usize - 1] - LEVEL_EXP[value as usize - 1];
              if exp > 0 {
                let text = if exp > 0 {
                  exp.to_formatted_string(&self.locale)
                } else {
                  String::new()
                };

                let response = Label::new(text).sense(Sense::click()).ui(ui);
                if response.on_hover_text_at_pointer("Click to copy").clicked() {
                  util::set_clipboard_contents(format!("{exp}"));
                }
              }
            },
          );

          ui.spacing_mut().item_spacing.x = x_spacing;
          ui.label("Next");

          ui.spacing_mut().item_spacing.x *= 0.5;
          ui.add(DragValue::new(value).clamp_range(LVL_RANGE));

          ui.spacing_mut().item_spacing.x = x_spacing;
          ui.label(RichText::from("Adv Lvl").color(LABEL_COLOR));
        });

        // Avatar combo-box.
        ui.add_enabled_ui(!self.avatars.is_empty(), |ui| {
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
            self.set_avatar(frame.storage_mut().expect(NONE_ERR), avatar)
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

  fn show_skill_category(&mut self, ui: &mut Ui, category: SkillCategory) {
    let (scroll_id, groups) = match category {
      SkillCategory::Adventurer => ("adventurer_skills", &self.adventurer_skills),
      SkillCategory::Producer => ("producer_skills", &self.producer_skills),
    };

    ui.vertical(|ui| {
      ui.add_enabled_ui(!self.avatar.is_empty(), |ui| {
        ScrollArea::vertical()
          .id_source(scroll_id)
          .always_show_scroll(true)
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
                      let mut plan = self.avatar_plan.lock().expect(FAIL_ERR);
                      for skill in &skill_group.skills {
                        let level = get_skill_lvl_mut(&mut plan.skill_lvls, skill.id);
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
                              if response.on_hover_text_at_pointer("Click to copy").clicked() {
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

  pub fn save(&self, storage: &mut dyn Storage) {
    let plan = self.avatar_plan.lock().expect(FAIL_ERR);
    config::set_avatar_plan(storage, &self.avatar, &plan);
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
      let avatars = future.await;
      tx.unbounded_send(avatars).expect(FAIL_ERR);
      ctx.request_repaint();
    };

    // Execute the future on a pooled thread.
    self.threads.spawn_ok(future);
  }

  fn set_avatar(&mut self, storage: &mut dyn Storage, avatar: String) {
    if self.avatar != avatar {
      let mut plan = self.avatar_plan.lock().expect(FAIL_ERR);

      // Store the values.
      config::set_avatar_plan(storage, &self.avatar, &plan);

      // Store the new avatar name.
      config::set_exp_avatar(storage, avatar.clone());

      // Get the values for the new avatar.
      *plan = config::get_avatar_plan(storage, &avatar).unwrap_or(AvatarPlan::new());

      self.avatar = avatar;
    }
  }
}

struct Channel {
  tx: mpsc::UnboundedSender<Vec<String>>,
  rx: mpsc::UnboundedReceiver<Vec<String>>,
  cancel_avatars: Option<Cancel>,
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
