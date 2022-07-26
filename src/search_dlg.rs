use crate::util::{self, AppState};
use eframe::{
  egui::{Context, Key, Layout, RichText, Window},
  emath::{Align, Align2},
  epaint::Color32,
};
use regex::Regex;
use std::sync::{atomic::Ordering, Arc};

pub struct SearchDlg {
  state: Arc<AppState>,
  title: String,
  text: String,
  error: String,
  search: Option<util::Search>,
  search_type: SearchType,
  visible: bool,
  focus: bool,
}

// Dialog window for inputting search term.
impl SearchDlg {
  pub fn new(state: Arc<AppState>) -> Self {
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
            let response = ui.text_edit_singleline(&mut self.text);
            if self.focus {
              self.focus = false;
              response.request_focus();
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
    self.state.enabled.store(false, Ordering::Relaxed);
    self.title = title;
    self.search = None;
    self.visible = true;
    self.focus = true;
  }

  pub fn take_search_term(&mut self) -> Option<util::Search> {
    self.search.take()
  }

  fn accept(&mut self) {
    if self.text.is_empty() {
      return;
    }

    self.search = match self.search_type {
      SearchType::Default | SearchType::NoCase => {
        let ignore_case = self.search_type == SearchType::NoCase;
        let mut find = String::new();
        std::mem::swap(&mut find, &mut self.text);
        Some(util::Search::String { find, ignore_case })
      }
      SearchType::Regex => match Regex::new(&self.text) {
        Ok(regex) => {
          self.text.clear();
          Some(util::Search::Regex(regex))
        }
        Err(err) => {
          self.text = format!("{:?}", err);
          return;
        }
      },
    };

    self.state.enabled.store(true, Ordering::Relaxed);
    self.title.clear();
    self.visible = false;
  }

  fn reject(&mut self) {
    self.state.enabled.store(true, Ordering::Relaxed);
    self.title.clear();
    self.text.clear();
    self.visible = false;
  }

  fn handle_hotkeys(&mut self, ctx: &Context) {
    if ctx.input().key_pressed(Key::Enter) {
      self.accept();
    } else if ctx.input().key_pressed(Key::Escape) {
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
