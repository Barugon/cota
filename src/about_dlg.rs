use crate::util::{self, AppState};
use eframe::{
  egui::{Context, Key, RichText, Window},
  emath::Align2,
  epaint::Color32,
};
use egui_extras::RetainedImage;

pub struct AboutDlg {
  logo: RetainedImage,
  state: AppState,
  visible: bool,
}

impl AboutDlg {
  pub fn new(state: AppState) -> Self {
    let logo_id = format!("{}_logo", util::APP_NAME);
    let logo =
      RetainedImage::from_image_bytes(logo_id, util::APP_ICON).expect("Unable to decode app icon");
    let visible = false;

    Self {
      logo,
      state,
      visible,
    }
  }

  pub fn show(&mut self, ctx: &Context) -> bool {
    if self.visible {
      self.handle_hotkeys(ctx);

      let available = ctx.available_rect();
      let mut open = true;

      Window::new(RichText::from(format!("👍  About {}", util::APP_NAME)).strong())
        .open(&mut open)
        .collapsible(false)
        .current_pos([0.0, 24.0])
        .anchor(Align2::CENTER_TOP, [0.0, 0.0])
        .default_size([available.width(), 0.0])
        .resizable(false)
        .show(ctx, |ui| {
          ui.add_space(8.0);
          ui.vertical_centered(|ui| {
            self.logo.show_scaled(ui, 0.5);
            ui.add_space(4.0);
            ui.label(
              RichText::new(util::APP_TITLE)
                .heading()
                .color(Color32::GOLD),
            );
            ui.label(format!("Version {}", util::APP_VERSION));
            ui.label(format!("Copyright © 2022 {}", util::APP_AUTHORS));
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
    if ctx.input().key_pressed(Key::Escape) {
      self.close();
    }
  }
}
