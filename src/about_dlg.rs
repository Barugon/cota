use crate::util::{AppState, APP_AUTHORS, APP_ICON, APP_NAME, APP_TITLE, APP_VERSION};
use eframe::{
  egui,
  emath::Align2,
  epaint::{Color32, ColorImage, TextureHandle, Vec2},
};
use egui::{Context, Key, RichText, Ui, Window};

pub struct AboutDlg {
  logo: Option<(Vec2, TextureHandle)>,
  state: AppState,
  visible: bool,
}

impl AboutDlg {
  pub fn new(state: AppState) -> Self {
    Self {
      logo: None,
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
            self.draw_logo(ui);
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

  fn draw_logo(&mut self, ui: &mut Ui) {
    if self.logo.is_none() {
      let logo_id = format!("{APP_NAME}_logo");
      let image = image::load_from_memory(APP_ICON).unwrap();
      let size = [image.width() as _, image.height() as _];
      let pixels = image.to_rgba8();
      let pixels = pixels.as_flat_samples();
      let image = ColorImage::from_rgba_unmultiplied(size, pixels.as_slice());
      let size = Vec2::new(size[0] as f32, size[1] as f32);
      let texture = ui.ctx().load_texture(logo_id, image, Default::default());
      self.logo = Some((size, texture));
    }

    let (size, texture) = self.logo.as_ref().unwrap();
    ui.image((texture.id(), *size * 0.5));
  }
}
