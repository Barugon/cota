use crate::{
  ethos::{Siege, CABALISTS, TOWNS, VIRTUES},
  util::AppState,
};
use eframe::{
  egui::{Context, Grid, Key, RichText, WidgetText, Window},
  emath::Align2,
  epaint::Color32,
};

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
        Grid::new("towns_grid").show(ui, |ui| {
          // Header.
          const HEADER_COLOR: Color32 = Color32::from_rgb(229, 187, 123);
          ui.label(RichText::from("Town (Devotional)  ").color(HEADER_COLOR));
          ui.label(RichText::from("Cabalist  ").color(HEADER_COLOR));
          ui.label(RichText::from("Remaining Time  ").color(HEADER_COLOR));
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
                  if first {
                    first = false;
                  } else {
                    // Show an empty label for the town.
                    ui.label(WidgetText::default());
                  }

                  ui.label(format!("{}  ", CABALISTS[cabalist_index]));
                  ui.label(get_countdown_text(sieges[cabalist_index].remain_secs()));
                  ui.end_row();
                }
              }
            } else {
              // Show the town with the inactive color.
              let text = format!("{} ({:?})  ", TOWNS[town_index], VIRTUES[town_index]);
              ui.label(RichText::from(text).color(Color32::from_rgb(102, 154, 180)));
              ui.label("None");

              // TODO: find next siege.

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

/// Get the remaining time in HH:MM:SS format.
fn get_countdown_text(sec: i32) -> String {
  let min = sec / 60;
  let sec = sec % 60;
  let hour = min / 60;
  let min = min % 60;
  return format!("{hour:02}:{min:02}:{sec:02}");
}
