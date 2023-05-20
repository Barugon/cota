use crate::{
  plant_info::{self, Environment, Plant, Seed},
  util::{AppState, NONE_ERR},
};
use chrono::{Local, NaiveDate, NaiveTime, Timelike};
use eframe::{
  egui::{ComboBox, Context, DragValue, Key, RichText, TextEdit, Window},
  emath::Align2,
  epaint::Color32,
};
use egui_extras::DatePickerButton;

#[derive(Default)]
pub struct PlantDlg {
  state: AppState,
  date: NaiveDate,
  hour: u32,
  min: u32,
  seed_types: Vec<Seed>,
  seed_names: Vec<&'static str>,
  seed_index: Option<usize>,
  environment: Option<Environment>,
  description: String,
  result: Option<Plant>,
  visible: bool,
}

impl PlantDlg {
  pub fn new(state: AppState) -> Self {
    let seeds = plant_info::parse_seeds();
    let seed_types = seeds.iter().map(|seed| seed.1).collect();
    let seed_names = seeds.iter().map(|seed| seed.0).collect();
    Self {
      state,
      seed_types,
      seed_names,
      ..Default::default()
    }
  }

  pub fn open(&mut self) {
    if !self.visible {
      let now = Local::now();
      self.date = now.date_naive();
      self.hour = now.hour();
      self.min = now.minute();
      self.result = None;
      self.state.set_disabled(true);
      self.visible = true;
    }
  }

  pub fn show(&mut self, ctx: &Context) -> bool {
    if self.visible {
      self.handle_hotkeys(ctx);

      let available = ctx.available_rect();
      let mut open = true;

      Window::new(RichText::from("⏰  Add Timer").strong())
        .open(&mut open)
        .collapsible(false)
        .current_pos([0.0, 24.0])
        .anchor(Align2::CENTER_TOP, [0.0, 0.0])
        .default_size([available.width(), 200.0])
        .resizable(false)
        .show(ctx, |ui| {
          const LABEL_COLOR: Color32 = Color32::from_rgb(154, 187, 154);
          let item_spacing = ui.spacing().item_spacing;

          ui.horizontal(|ui| {
            // Seed.
            ui.spacing_mut().item_spacing.x = item_spacing.x * 0.5;
            ui.label(RichText::from("Seed").color(LABEL_COLOR));
            let text = if let Some(index) = self.seed_index {
              self.seed_names[index]
            } else {
              Default::default()
            };
            ui.spacing_mut().item_spacing.x = item_spacing.x;
            ComboBox::from_id_source("seed_combo")
              .selected_text(text)
              .width(157.0)
              .show_ui(ui, |ui| {
                for index in 0..self.seed_names.len() {
                  let text = self.seed_names[index];
                  let selected = Some(index) == self.seed_index;
                  if ui.selectable_label(selected, text).clicked() && !selected {
                    self.seed_index = Some(index);
                  }
                }
              });

            // Environment.
            ui.spacing_mut().item_spacing.x = item_spacing.x * 0.5;
            ui.label(RichText::from("Env").color(LABEL_COLOR));
            let text = if let Some(environment) = self.environment {
              format!("{environment:?}")
            } else {
              Default::default()
            };
            ui.spacing_mut().item_spacing.x = item_spacing.x;
            ComboBox::from_id_source("environment_combo")
              .selected_text(text)
              .show_ui(ui, |ui| {
                let selected = self.environment == Some(Environment::Greenhouse);
                if ui.selectable_label(selected, "Greenhouse").clicked() && !selected {
                  self.environment = Some(Environment::Greenhouse);
                }

                let selected = self.environment == Some(Environment::Outside);
                if ui.selectable_label(selected, "Outside").clicked() && !selected {
                  self.environment = Some(Environment::Outside);
                }

                let selected = self.environment == Some(Environment::Inside);
                if ui.selectable_label(selected, "Inside").clicked() && !selected {
                  self.environment = Some(Environment::Inside);
                }
              });

            // Date.
            let widget = DatePickerButton::new(&mut self.date)
              .id_source("plant_date_picker")
              .show_icon(false);
            ui.spacing_mut().item_spacing.x = item_spacing.x * 0.5;
            ui.add(widget);

            // Hour.
            ui.spacing_mut().interact_size.x = 23.0;
            let widget = DragValue::new(&mut self.hour)
              .custom_formatter(|val, _| format!("{val:02}"))
              .clamp_range(0..=23)
              .speed(0.125);
            ui.spacing_mut().item_spacing.x = 1.0;
            ui.add(widget);
            ui.label(":");

            // Minute.
            let widget = DragValue::new(&mut self.min)
              .custom_formatter(|val, _| format!("{val:02}"))
              .clamp_range(0..=59)
              .speed(0.125);
            ui.spacing_mut().item_spacing.x = item_spacing.x;
            ui.add(widget);
          });

          ui.add_space(3.0);

          ui.horizontal(|ui| {
            // Additional information.
            let widget = TextEdit::singleline(&mut self.description).hint_text("additional info");
            ui.spacing_mut().item_spacing.x = item_spacing.x;
            ui.add_sized(ui.available_size(), widget);
          });

          ui.separator();
          ui.horizontal(|ui| {
            let enabled = self.seed_index.is_some() && self.environment.is_some();
            ui.add_enabled_ui(enabled, |ui| {
              if ui.button("Ok").clicked() {
                self.accept();
              }
            });

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

  pub fn take_result(&mut self) -> Option<Plant> {
    self.result.take()
  }

  fn accept(&mut self) {
    if self.visible {
      let Some(index) = self.seed_index else { return };
      let Some(environment) = self.environment else { return };
      let time = NaiveTime::from_hms_opt(self.hour, self.min, 0).expect(NONE_ERR);
      self.result = Some(Plant::new(
        self.description.clone(),
        self.date.and_time(time),
        self.seed_names[index].to_owned(),
        self.seed_types[index],
        environment,
      ));
      self.state.set_disabled(false);
      self.visible = false;
    }
  }

  fn reject(&mut self) {
    if self.visible {
      self.state.set_disabled(false);
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
