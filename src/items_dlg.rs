use crate::{game_data::Item, util::AppState};
use eframe::{
  egui::{Context, DragValue, Key, Layout, RichText, TextStyle, Window},
  emath::{Align, Align2},
  epaint::Color32,
};
use egui_extras::{Size, TableBuilder};
use std::sync::{atomic::Ordering, Arc};

pub struct ItemsDlg {
  state: Arc<AppState>,
  visible: bool,
}

impl ItemsDlg {
  pub fn new(state: Arc<AppState>) -> Self {
    Self {
      state,
      visible: false,
    }
  }

  pub fn show(&mut self, items: &mut Vec<Item>, ctx: &Context) -> bool {
    let mut modified = false;
    if self.visible {
      self.handle_hotkeys(ctx);

      let available = ctx.available_rect();
      let mut open = true;

      Window::new(RichText::from("🎒  Inventory Items").strong())
        .open(&mut open)
        .collapsible(false)
        .resizable(false)
        .current_pos([0.0, 24.0])
        .anchor(Align2::CENTER_TOP, [0.0, 0.0])
        .default_size(available.size())
        .show(ctx, |ui| {
          // This add_visible_ui is here only to constrain the set_max_height call.
          ui.add_visible_ui(true, |ui| {
            ui.set_max_height(available.height() * 0.8);
            let row_size = TextStyle::Body.resolve(ui.style()).size + 4.0;
            TableBuilder::new(ui)
              .cell_layout(Layout::left_to_right(Align::Center))
              .striped(true)
              .column(Size::relative(0.75))
              .column(Size::relative(0.11))
              .column(Size::remainder())
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
                      ui.label(RichText::from(&item.name).color(NAME_COLOR));
                    });
                    row.col(|ui| {
                      if !item.bag {
                        // It's safe to adjust the stack size for all items (except containers) even for those
                        // that are equipped or have durability.
                        let speed = (item.cnt as f64 / 100.0).max(1.0);
                        let range = 1..=i16::MAX;
                        let widget = DragValue::new(&mut item.cnt)
                          .speed(speed)
                          .clamp_range(range);
                        if ui.add(widget).changed() {
                          modified = true;
                        }
                      }
                    });
                    row.col(|ui| {
                      if let Some(dur) = &mut item.dur {
                        ui.set_enabled(dur.minor != dur.major);
                        if ui.button("Repair").clicked() {
                          // The actual maximum durability is unknown here, so just set the durability to a high value.
                          // The value will be adjusted in-game to the actual maximum when the item takes damage.
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
      self.state.disable.store(true, Ordering::Relaxed);
      self.visible = true;
    }
  }

  pub fn close(&mut self) {
    if self.visible {
      self.state.disable.store(false, Ordering::Relaxed);
      self.visible = false;
    }
  }

  fn handle_hotkeys(&mut self, ctx: &Context) {
    if ctx.input().key_pressed(Key::Escape) {
      self.close();
    }
  }
}
