use crate::{game_data::Item, util};
use eframe::{
  egui::{Context, DragValue, Key, Layout, RichText, Window},
  emath::{Align, Align2},
  epaint::Color32,
};
use egui_extras::{Column, TableBuilder};
use util::AppState;

pub struct ItemsDlg {
  state: AppState,
  visible: bool,
}

impl ItemsDlg {
  pub fn new(state: AppState) -> Self {
    Self { state, visible: false }
  }

  pub fn show(&mut self, items: &mut Vec<Item>, ctx: &Context) -> bool {
    let mut modified = false;
    if self.visible {
      self.handle_hotkeys(ctx);

      let available = ctx.available_rect();
      let mut open = true;

      Window::new(RichText::from("⚔  Inventory Items").strong())
        .open(&mut open)
        .collapsible(false)
        .resizable(false)
        .current_pos([0.0, 24.0])
        .anchor(Align2::CENTER_TOP, [0.0, 0.0])
        .default_size(available.size())
        .show(ctx, |ui| {
          // This scope is here to constrain the set_max_height call.
          ui.scope(|ui| {
            ui.set_max_height(available.height() * 0.8);
            let spacing = ui.spacing().item_spacing;
            let row_size = util::button_size(ui) + spacing[1] * 2.0;
            let available_width = ui.available_width() - util::scroll_bar_size(ui);
            TableBuilder::new(ui)
              .cell_layout(Layout::left_to_right(Align::Center))
              .striped(true)
              .column(Column::exact(available_width * 0.75 - spacing[0]))
              .column(Column::exact(available_width * 0.125 - spacing[0]))
              .column(Column::remainder())
              .header(row_size, |mut header| {
                const HEADER_COLOR: Color32 = Color32::from_rgb(229, 187, 123);
                header.col(|ui| {
                  ui.label(RichText::from("Item Name").color(HEADER_COLOR));
                });
                header.col(|ui| {
                  ui.label(RichText::from("Count").color(HEADER_COLOR));
                });
                header.col(|_| {});
              })
              .body(|mut body| {
                const NAME_COLOR: Color32 = Color32::from_rgb(154, 187, 154);
                for item in items {
                  body.row(row_size, |mut row| {
                    row.col(|ui| {
                      ui.label(RichText::from(item.name()).color(NAME_COLOR));
                    });
                    row.col(|ui| {
                      if !item.is_container() {
                        // It's safe to adjust the count (stack size) for all items (except containers) even for
                        // those that are equipped or have durability.
                        let count = item.count_mut();
                        let speed = (*count as f64 / 100.0).max(1.0);
                        let range = 1..=i16::MAX;
                        let widget = DragValue::new(count).speed(speed).range(range);
                        if ui.add(widget).changed() {
                          modified = true;
                        }
                      }
                    });
                    row.col(|ui| {
                      if let Some(dur) = item.durability_mut() {
                        if dur.minor == dur.major {
                          ui.disable();
                        }

                        if ui.button("Repair").clicked() {
                          // The actual maximum durability is unknown here so just set the durability to a high value;
                          // it will be adjusted in-game to the actual maximum when the item takes damage.
                          dur.minor = 5000.0;
                          dur.major = 5000.0;
                          modified = true;
                        }
                      }
                    });
                  });
                }
              });
          });

          ui.add_space(ui.spacing().item_spacing.y);
          ui.separator();
          ui.horizontal(|ui| {
            if ui.button("Close").clicked() {
              self.close();
            }
          });
        });
      if !open {
        self.close();
      }
    }
    modified
  }

  pub fn open(&mut self) {
    if !self.visible {
      self.state.set_disabled(true);
      self.visible = true;
    }
  }

  pub fn close(&mut self) {
    if self.visible {
      self.state.set_disabled(false);
      self.visible = false;
    }
  }

  fn handle_hotkeys(&mut self, ctx: &Context) {
    if ctx.input(|state| state.key_pressed(Key::Escape)) {
      self.close();
    }
  }
}
