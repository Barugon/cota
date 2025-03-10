use crate::util::{APP_AUTHORS, APP_ICON, APP_NAME, APP_TITLE, APP_VERSION, AppState, Picture};
use eframe::{egui, emath::Align2, epaint::Color32};
use egui::{Context, Key, RichText, Window};

pub struct AboutDlg {
  logo: Picture,
  state: AppState,
  visible: bool,
}

impl AboutDlg {
  pub fn new(state: AppState) -> Self {
    Self {
      logo: Picture::new(format!("{APP_NAME}_logo"), APP_ICON),
      state,
      visible: false,
    }
  }

  pub fn show(&mut self, ctx: &Context) -> bool {
    if self.visible {
      self.handle_hotkeys(ctx);

      let available = ctx.available_rect();
      let mut open = true;

      Window::new(RichText::from("👍  About CotA").strong())
        .open(&mut open)
        .collapsible(false)
        .current_pos([0.0, 24.0])
        .anchor(Align2::CENTER_TOP, [0.0, 0.0])
        .default_size([available.width(), 0.0])
        .resizable(false)
        .show(ctx, |ui| {
          ui.add_space(8.0);
          ui.vertical_centered(|ui| {
            ui.image((self.logo.texture_id(ctx), self.logo.size() * 0.5));
            ui.add_space(4.0);
            ui.label(RichText::new(APP_TITLE).heading().color(Color32::GOLD));
            ui.label(format!("Version {APP_VERSION}"));
            ui.label(format!("Copyright © 2022-present {APP_AUTHORS}"));
          });
          ui.add_space(8.0);
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

    self.visible
  }

  pub fn open(&mut self) {
    if !self.visible {
      self.state.set_disabled(true);
      self.visible = true;
    }
  }

  fn close(&mut self) {
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
