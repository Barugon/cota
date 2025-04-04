use std::mem;

use crate::util::{AppState, Search};
use eframe::{
  egui::{
    Context, Key, Layout, RichText, TextEdit, Window,
    text::{CCursor, CCursorRange},
  },
  emath::{Align, Align2},
  epaint::Color32,
};
use regex::Regex;

pub struct SearchDlg {
  state: AppState,
  title: String,
  text: String,
  error: String,
  search: Option<Search>,
  search_type: SearchType,
  visible: bool,
  focus: bool,
}

// Dialog window for inputting search term.
impl SearchDlg {
  pub fn new(state: AppState) -> Self {
    Self {
      state,
      title: String::new(),
      text: String::new(),
      error: String::new(),
      search: None,
      search_type: SearchType::Default,
      visible: false,
      focus: false,
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
        .default_size([available.width(), 0.0])
        .show(ctx, |ui| {
          ui.vertical_centered_justified(|ui| {
            let mut output = TextEdit::singleline(&mut self.text).show(ui);
            if mem::take(&mut self.focus) {
              output.response.request_focus();
              if !self.text.is_empty() {
                // Select the text.
                let select = CCursorRange::two(CCursor::new(0), CCursor::new(self.text.len()));
                output.state.cursor.set_char_range(Some(select));
                output.state.store(ui.ctx(), output.response.id);
              }
            }
          });
          if !self.error.is_empty() {
            ui.vertical_centered(|ui| {
              let error = RichText::new(&self.error).color(Color32::LIGHT_RED);
              ui.label(error);
            });
          }
          ui.separator();
          ui.horizontal(|ui| {
            ui.add_enabled_ui(!self.text.is_empty(), |ui| {
              if ui.button("OK").clicked() {
                self.accept();
              }
            });

            if ui.button("Cancel").clicked() {
              self.reject();
            }

            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
              let widget = ui.radio(self.search_type == SearchType::Regex, "Regex");
              if widget.clicked() {
                self.search_type = match self.search_type {
                  SearchType::Regex => SearchType::Default,
                  _ => SearchType::Regex,
                };
              }

              let widget = ui.radio(self.search_type == SearchType::NoCase, "Ignore Case");
              if widget.clicked() {
                self.search_type = match self.search_type {
                  SearchType::NoCase => SearchType::Default,
                  _ => SearchType::NoCase,
                };
              }
            });
          });
        });
      if !open {
        self.reject();
      }
    }
    self.visible
  }

  pub fn open(&mut self, title: String) {
    if !self.visible {
      self.state.set_disabled(true);
      self.title = title;
      self.search = None;
      self.visible = true;
      self.focus = true;
    }
  }

  pub fn take_search_term(&mut self) -> Option<Search> {
    self.search.take()
  }

  fn accept(&mut self) {
    if self.visible {
      if self.text.is_empty() {
        return;
      }

      self.search = match self.search_type {
        SearchType::Default | SearchType::NoCase => {
          let ignore_case = self.search_type == SearchType::NoCase;
          let find = self.text.clone().into();
          Some(Search::String { find, ignore_case })
        }
        SearchType::Regex => match Regex::new(&self.text) {
          Ok(regex) => Some(Search::Regex(regex)),
          Err(err) => {
            self.text = format!("{err:?}");
            return;
          }
        },
      };

      self.state.set_disabled(false);
      self.title.clear();
      self.visible = false;
    }
  }

  fn reject(&mut self) {
    if self.visible {
      self.state.set_disabled(false);
      self.title.clear();
      self.visible = false;
    }
  }

  fn handle_hotkeys(&mut self, ctx: &Context) {
    if ctx.input(|state| state.key_pressed(Key::Enter)) {
      self.accept();
    } else if ctx.input(|state| state.key_pressed(Key::Escape)) {
      self.reject();
    }
  }
}

#[derive(Eq, PartialEq)]
enum SearchType {
  Default,
  NoCase,
  Regex,
}
