use crate::{log_data, util};
use eframe::{
  egui::{
    scroll_area::ScrollBarVisibility, Context, Key, RichText, ScrollArea, TextEdit, TextFormat, Ui,
    Window,
  },
  emath::Align2,
  epaint::{
    text::{LayoutJob, LayoutSection},
    Color32, FontFamily, FontId,
  },
};
use util::{AppState, Cancel, Search};

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
                TextEdit::multiline(&mut text).layouter(&mut |ui: &Ui, _text: &str, wrap: f32| {
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
      self.state.set_disabled(false);
      self.title = format!("🗊  Search Results ({avatar})");
      self.status = RichText::from("Processing...").color(Color32::from_rgb(229, 187, 123));
      self.cancel = Some(cancel);
      self.visible = true;
      self.init = true;
    }
  }

  pub fn set_text(&mut self, text: String, search: Search, ctx: &Context) {
    if self.visible {
      if text.is_empty() {
        self.layout = None;
        self.status = RichText::from("Nothing Found").color(Color32::from_rgb(229, 187, 123));
      } else {
        let font = FontId::new(14.0, FontFamily::Monospace);
        let color = ctx.style().visuals.text_color();
        self.layout = Some(layout_text(text, search, font, color));
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

/// Construct a `LayoutJob` for highlighted results.
fn layout_text(text: String, search: Search, font: FontId, color: Color32) -> LayoutJob {
  let mut sections = Vec::new();
  for line in text.lines() {
    // Highlight the date/time.
    if let Some(date) = log_data::get_log_date(line) {
      const DATE_COLOR: Color32 = Color32::from_rgb(180, 154, 102);
      let pos = util::offset(&text, date).unwrap();
      sections.push(LayoutSection {
        leading_space: 0.0,
        byte_range: pos..pos + date.len(),
        format: TextFormat::simple(font.clone(), DATE_COLOR),
      });
    }

    let mut line = log_data::get_log_text(line);
    loop {
      let pos = util::offset(&text, line).unwrap();
      if let Some(find) = search.find_in(line) {
        let start = pos + find.start;
        let end = pos + find.end;
        if start > pos {
          // Text before the match.
          sections.push(LayoutSection {
            leading_space: 0.0,
            byte_range: pos..start,
            format: TextFormat::simple(font.clone(), color),
          });
        }

        const MATCH_COLOR: Color32 = Color32::from_rgb(102, 154, 180);

        // Highlight the match
        sections.push(LayoutSection {
          leading_space: 0.0,
          byte_range: start..end,
          format: TextFormat::simple(font.clone(), MATCH_COLOR),
        });

        // Move past the match.
        line = &line[find.end..];
      } else {
        if !line.is_empty() {
          // The rest.
          sections.push(LayoutSection {
            leading_space: 0.0,
            byte_range: pos..pos + line.len() + 1,
            format: TextFormat::simple(font.clone(), color),
          });
        }
        break;
      }
    }
  }

  LayoutJob {
    text,
    sections,
    break_on_newline: true,
    ..Default::default()
  }
}
