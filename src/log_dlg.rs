use crate::util::{AppState, Cancel};
use eframe::{
  egui::{Context, Key, RichText, ScrollArea, TextBuffer, TextEdit, Ui, Window, scroll_area::ScrollBarVisibility},
  emath::Align2,
  epaint::{Color32, text::LayoutJob},
};

pub struct LogDlg {
  title: String,
  state: AppState,
  cancel: Option<Cancel>,
  status: RichText,
  layout: Option<LayoutJob>,
  visible: bool,
  init: bool,
}

/// Dialog window for showing log search results.
impl LogDlg {
  pub fn new(state: AppState) -> Self {
    Self {
      title: String::new(),
      state,
      cancel: None,
      status: Default::default(),
      layout: None,
      visible: false,
      init: false,
    }
  }

  pub fn show(&mut self, ctx: &Context) {
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
        .default_size(available.size())
        .show(ctx, |ui| {
          if !self.status.is_empty() {
            ui.horizontal(|ui| {
              ui.centered_and_justified(|ui| {
                ui.label(self.status.clone());
              });
            });
          } else if let Some(layout_job) = &self.layout {
            // Display the text as selectable but not editable.
            let mut text = layout_job.text.as_str();
            if self.init {
              self.init = false;
              ScrollArea::vertical().vertical_scroll_offset(0.0)
            } else {
              ScrollArea::vertical()
            }
            .max_height(available.height() * 0.75)
            .scroll_bar_visibility(ScrollBarVisibility::AlwaysVisible)
            .show(ui, |ui| {
              ui.add_sized(
                ui.available_size(),
                TextEdit::multiline(&mut text).layouter(&mut |ui: &Ui, _text: &dyn TextBuffer, wrap: f32| {
                  let mut layout_job = layout_job.clone();
                  layout_job.wrap.max_width = wrap;
                  ui.fonts(|fonts| fonts.layout_job(layout_job))
                }),
              );
            });
          }

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
  }

  pub fn open(&mut self, avatar: &str, cancel: Cancel) {
    if !self.visible {
      self.state.set_disabled(true);
      self.title = format!("🗊  Search Results ({avatar})");
      self.status = RichText::from("Processing...").color(Color32::from_rgb(229, 187, 123));
      self.cancel = Some(cancel);
      self.visible = true;
      self.init = true;
    }
  }

  pub fn set_layout(&mut self, layout: LayoutJob, ctx: &Context) {
    if self.visible {
      if layout.text.is_empty() {
        self.layout = None;
        self.status = RichText::from("Nothing Found").color(Color32::from_rgb(229, 187, 123));
      } else {
        self.layout = Some(layout);
        self.status = Default::default();
      }
      ctx.request_repaint();
    }
  }

  fn close(&mut self) {
    if self.visible {
      if let Some(mut cancel) = self.cancel.take() {
        // Cancel the search if it's still outstanding.
        cancel.cancel();
      }

      self.state.set_disabled(false);
      self.status = Default::default();
      self.layout = None;
      self.visible = false;
    }
  }

  fn handle_hotkeys(&mut self, ctx: &Context) {
    if ctx.input(|state| state.key_pressed(Key::Escape)) {
      self.close();
    }
  }
}
