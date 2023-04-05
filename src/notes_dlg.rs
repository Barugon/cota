use crate::util::AppState;
use eframe::{
  egui::{
    scroll_area::ScrollBarVisibility, text::CCursor, text_edit::CCursorRange, Context, Key,
    RichText, ScrollArea, TextEdit, Window,
  },
  emath::Align2,
};

pub struct NotesDlg {
  state: AppState,
  title: String,
  text: String,
  result: Option<String>,
  visible: bool,
  init: bool,
}

// Dialog window for editing avatar notes.
impl NotesDlg {
  pub fn new(state: AppState) -> Self {
    Self {
      state,
      title: String::new(),
      text: String::new(),
      result: None,
      visible: false,
      init: false,
    }
  }

  pub fn show(&mut self, ctx: &Context) -> bool {
    if self.visible {
      self.handle_hotkeys(ctx);

      let available = ctx.available_rect();
      let mut open = true;

      Window::new(RichText::from(&self.title).strong())
        .open(&mut open)
        .collapsible(false)
        .resizable(false)
        .current_pos([0.0, 24.0])
        .anchor(Align2::CENTER_TOP, [0.0, 0.0])
        .default_width(available.width())
        .show(ctx, |ui| {
          if self.init {
            ScrollArea::vertical().vertical_scroll_offset(0.0)
          } else {
            ScrollArea::vertical()
          }
          .max_height(available.height() * 0.5)
          .scroll_bar_visibility(ScrollBarVisibility::AlwaysVisible)
          .show(ui, |ui| {
            let response = ui.add_sized(
              ui.available_size(),
              TextEdit::multiline(&mut self.text).code_editor(),
            );

            if self.init {
              self.init = false;

              // Set the cursor pos to the beginning.
              if let Some(mut state) = TextEdit::load_state(ctx, response.id) {
                let ccursor = CCursor::new(0);
                state.set_ccursor_range(Some(CCursorRange::one(ccursor)));
                state.store(ctx, response.id);
              }

              // Request focus.
              response.request_focus();
            }
          });
          ui.separator();
          ui.horizontal(|ui| {
            if ui.button("OK").clicked() {
              self.accept();
            }

            if ui.button("Cancel").clicked() {
              self.reject();
            }
          });
        });
      if !open {
        self.reject();
      }
    }
    self.visible
  }

  pub fn open(&mut self, avatar: &str, text: String) {
    if !self.visible {
      self.state.set_disabled(true);
      self.title = format!("📓  Notes for {avatar}");
      self.text = text;
      self.result = None;
      self.visible = true;
      self.init = true;
    }
  }

  pub fn take_text(&mut self) -> Option<String> {
    self.result.take()
  }

  fn accept(&mut self) {
    if self.visible {
      self.state.set_disabled(false);
      let mut text = String::new();
      std::mem::swap(&mut text, &mut self.text);
      self.result = Some(text);
      self.visible = false;
    }
  }

  fn reject(&mut self) {
    if self.visible {
      self.state.set_disabled(false);
      self.text.clear();
      self.visible = false;
    }
  }

  fn handle_hotkeys(&mut self, ctx: &Context) {
    if ctx.input(|state| state.key_pressed(Key::Escape)) {
      self.reject();
    }
  }
}
