use crate::{
  ethos::{Siege, CABALISTS, TOWNS, VIRTUES},
  util::{self, AppState},
};
use eframe::{
  egui::{Context, Grid, Key, Layout, RichText, WidgetText, Window},
  emath::{Align, Align2},
  epaint::Color32,
};
use std::mem;

pub struct TownsDlg {
  state: AppState,
  visible: bool,
}

impl TownsDlg {
  pub fn new(state: AppState) -> Self {
    Self {
      state,
      visible: false,
    }
  }

  pub fn open(&mut self) {
    self.state.set_disabled(true);
    self.visible = true;
  }

  pub fn show(&mut self, ctx: &Context, sieges: &[Siege]) {
    if !self.visible {
      return;
    }

    let towns = {
      let mut towns: [(bool, [bool; CABALISTS.len()]); TOWNS.len()] = Default::default();
      for (cabalist_index, siege) in sieges.iter().enumerate() {
        let town_index = siege.virtue() as usize;
        towns[town_index].0 = true;
        towns[town_index].1[cabalist_index] = true;
      }
      towns
    };

    self.handle_hotkeys(ctx);

    let available = ctx.available_rect();
    let mut open = true;

    Window::new(RichText::from("⚔  Town Sieges").strong())
      .open(&mut open)
      .collapsible(false)
      .current_pos([0.0, 24.0])
      .anchor(Align2::CENTER_TOP, [0.0, 0.0])
      .default_size([available.width(), 0.0])
      .resizable(false)
      .show(ctx, |ui| {
        Grid::new("towns_grid")
          .min_col_width((ui.available_width() - ui.spacing().item_spacing.x * 2.0) / 3.0)
          .show(ui, |ui| {
            // Header.
            const HEADER_COLOR: Color32 = Color32::from_rgb(229, 187, 123);
            ui.label(RichText::from("Town (Devotional)  ").color(HEADER_COLOR));
            ui.centered_and_justified(|ui| {
              ui.label(RichText::from("Cabalist  ").color(HEADER_COLOR));
            });
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
              ui.label(RichText::from("Remaining Time  ").color(HEADER_COLOR));
            });
            ui.end_row();

            for (town_index, info) in towns.iter().enumerate() {
              if info.0 {
                // Show the town with the active color.
                let text = format!("{} ({:?})  ", TOWNS[town_index], VIRTUES[town_index]);
                ui.label(RichText::from(text).color(Color32::from_rgb(154, 229, 255)));

                // List all the cabalists that are currently sieging.
                let mut first = true;
                for (cabalist_index, sieging) in info.1.iter().enumerate() {
                  if *sieging {
                    if !mem::take(&mut first) {
                      // Show an empty label for the town.
                      ui.label(WidgetText::default());
                    }

                    ui.centered_and_justified(|ui| {
                      ui.label(format!("{}  ", CABALISTS[cabalist_index]));
                    });
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                      ui.label(util::get_countdown_text(
                        Default::default(),
                        sieges[cabalist_index].remain_secs(),
                      ));
                    });
                    ui.end_row();
                  }
                }
              } else {
                // Show the town with the inactive color.
                let text = format!("{} ({:?})  ", TOWNS[town_index], VIRTUES[town_index]);
                ui.label(RichText::from(text).color(Color32::from_rgb(102, 154, 180)));
                ui.end_row();
              }
            }
          });
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

  pub fn close(&mut self) {
    self.state.set_disabled(false);
    self.visible = false;
  }

  fn handle_hotkeys(&mut self, ctx: &Context) {
    if ctx.input(|state| state.key_pressed(Key::Enter) || state.key_pressed(Key::Escape)) {
      self.close();
    }
  }
}
