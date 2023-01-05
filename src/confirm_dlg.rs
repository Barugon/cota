use crate::util::AppState;
use eframe::{
  egui::{Context, Key, RichText, Window},
  emath::Align2,
  epaint::Color32,
};

#[derive(Clone, Copy)]
pub enum Choice {
  Save,
  Discard,
}

pub enum Hence {
  Load,
  Exit,
}

pub struct ConfirmDlg {
  file: String,
  state: AppState,
  choice: Option<Choice>,
  hence: Option<Hence>,
  visible: bool,
}

/// Dialog window asking the user what to do with save-game changes.
impl ConfirmDlg {
  pub fn new(state: AppState) -> Self {
    Self {
      file: String::new(),
      state,
      choice: None,
      hence: None,
      visible: false,
    }
  }

  pub fn show(&mut self, ctx: &Context) -> bool {
    if self.visible {
      self.handle_hotkeys(ctx);

      let available = ctx.available_rect();
      let mut open = true;

      Window::new(RichText::from(format!("⚠  {}", &self.file)).strong())
        .open(&mut open)
        .collapsible(false)
        .current_pos([0.0, 24.0])
        .anchor(Align2::CENTER_TOP, [0.0, 0.0])
        .default_size([available.width(), 0.0])
        .resizable(false)
        .show(ctx, |ui| {
          ui.add_space(8.0);
          ui.vertical_centered(|ui| {
            let text = RichText::from(format!(
              "Changes have been made to {}.\nWhat would you like to do?",
              self.file
            ))
            .color(Color32::LIGHT_RED);
            ui.label(text);
          });
          ui.add_space(8.0);
          ui.separator();
          ui.horizontal(|ui| {
            if ui.button("Save").clicked() {
              self.close(Some(Choice::Save));
            }
            if ui.button("Discard").clicked() {
              self.close(Some(Choice::Discard));
            }
          });
        });
      if !open {
        self.close(None);
      }
    }

    self.visible
  }

  pub fn open(&mut self, file: String, hence: Hence) {
    if !self.visible {
      self.state.set_disabled(false);
      self.file = file;
      self.hence = Some(hence);
      self.choice = None;
      self.visible = true;
    }
  }

  pub fn visible(&self) -> bool {
    self.visible
  }

  pub fn take_choice(&mut self) -> Option<Choice> {
    self.choice.take()
  }

  pub fn take_hence(&mut self) -> Option<Hence> {
    self.hence.take()
  }

  fn close(&mut self, choice: Option<Choice>) {
    if self.visible {
      self.state.set_disabled(false);
      if choice.is_none() {
        // If choice is None then hence is None.
        self.hence = None;
      }
      self.choice = choice;
      self.visible = false;
    }
  }

  fn handle_hotkeys(&mut self, ctx: &Context) {
    if ctx.input().key_pressed(Key::Enter) {
      self.close(Some(Choice::Save));
    } else if ctx.input().key_pressed(Key::Escape) {
      self.close(None);
    }
  }
}
