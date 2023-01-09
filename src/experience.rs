use crate::util::{self, SkillInfo, SkillCategory, SkillInfoGroup};
use clipboard::{ClipboardContext, ClipboardProvider};
use eframe::{
  egui::{DragValue, Label, Layout, RichText, ScrollArea, Sense, Ui, Widget},
  emath::{Align, Vec2},
  epaint::Color32,
};
use egui_extras::{Column, TableBuilder};
use num_format::{Locale, ToFormattedString};

pub struct Experience {
  adventurer: Vec<SkillInfoGroup>,
  producer: Vec<SkillInfoGroup>,
  selected: SkillInfo,
  current_level: usize,
  target_level: usize,
  locale: Locale,
}

impl Experience {
  pub fn new() -> Self {
    Experience {
      adventurer: util::parse_skill_group(SkillCategory::Adventurer),
      producer: util::parse_skill_group(SkillCategory::Producer),
      selected: Default::default(),
      current_level: 1,
      target_level: 1,
      locale: util::get_locale(),
    }
  }

  pub fn show(&mut self, ui: &mut Ui) {
    const LABEL_COLOR: Color32 = Color32::from_rgb(154, 187, 154);

    // Tool bar.
    ui.horizontal(|ui| {
      ui.label(RichText::from("Current Level").color(LABEL_COLOR));
      let widget = DragValue::new(&mut self.current_level).clamp_range(util::LVL_RANGE);
      ui.add(widget);

      ui.label(RichText::from("Target Level").color(LABEL_COLOR));
      let widget = DragValue::new(&mut self.target_level).clamp_range(util::LVL_RANGE);
      ui.add(widget);

      ui.label(RichText::from("Experience").color(LABEL_COLOR));
      if let Some(exp) = self.get_needed_exp() {
        let (text, exp) = if exp < 0 {
          // Half experience returned for un-training.
          let exp = exp / 2;
          let text = format!("({})", exp.abs().to_formatted_string(&self.locale));
          let text = RichText::from(text).color(Color32::LIGHT_RED);
          (text, exp)
        } else {
          let text = exp.to_formatted_string(&self.locale);
          let text = RichText::from(text).color(Color32::WHITE);
          (text, exp)
        };
        let response = Label::new(text).sense(Sense::click()).ui(ui);
        if response.on_hover_text_at_pointer("Click to copy").clicked() {
          // Copy the value to the clipboard.
          if let Ok::<ClipboardContext, _>(mut ctx) = ClipboardProvider::new() {
            let _ = ctx.set_contents(format!("{}", exp));
          }
        }
      }
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
      SkillCategory::Adventurer => ("adventurer_skills", &self.adventurer),
      SkillCategory::Producer => ("producer_skills", &self.producer),
    };

    ui.vertical(|ui| {
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
                let available_width = ui.available_width() - util::scroll_bar_size(ui);
                TableBuilder::new(ui)
                  .cell_layout(Layout::left_to_right(Align::Center))
                  .striped(true)
                  .vscroll(false)
                  .column(Column::exact(available_width * 0.64))
                  .column(Column::exact(available_width * 0.18))
                  .column(Column::remainder())
                  .header(row_size, |mut header| {
                    const HEADER_COLOR: Color32 = Color32::from_rgb(229, 187, 123);
                    header.col(|ui| {
                      ui.label(RichText::from("Skill").color(HEADER_COLOR));
                    });
                    header.col(|ui| {
                      ui.label(RichText::from("Exp Mul").color(HEADER_COLOR));
                    });
                    header.col(|ui| {
                      ui.label(RichText::from("ID").color(HEADER_COLOR));
                    });
                  })
                  .body(|mut body| {
                    for skill in &skill_group.skills {
                      body.row(row_size, |mut row| {
                        row.col(|ui| {
                          // Use a single column so that the selectable label fills the entire table column.
                          ui.columns(1, |col| {
                            let is_selected = self.selected.id == skill.id;
                            let color = if is_selected {
                              const SELECTED_NAME_COLOR: Color32 = Color32::from_rgb(192, 222, 255);
                              SELECTED_NAME_COLOR
                            } else {
                              const NAME_COLOR: Color32 = Color32::from_rgb(102, 154, 180);
                              NAME_COLOR
                            };

                            let text = RichText::from(skill.name).color(color);
                            if col[0].selectable_label(is_selected, text).clicked() {
                              self.selected = skill.clone();
                            }
                          });
                        });
                        row.col(|ui| {
                          ui.label(format!("{}x", skill.mul));
                        });
                        row.col(|ui| {
                          ui.label(format!("{}", skill.id));
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
  }

  fn get_needed_exp(&self) -> Option<i64> {
    if self.selected.name.is_empty() {
      return None;
    }
    let val = util::SKILL_EXP[self.target_level - 1] - util::SKILL_EXP[self.current_level - 1];
    Some((val as f64 * self.selected.mul).ceil() as i64)
  }
}
