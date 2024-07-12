use crate::{
  log_data::{self, DPSTally, Span},
  util::{AppState, Cancel},
};
use chrono::{Local, NaiveDateTime, NaiveTime, Timelike};
use eframe::{
  egui::{Context, DragValue, Grid, Key, RichText, Ui, Window},
  emath::Align2,
  epaint::Color32,
};
use egui_extras::DatePickerButton;
use futures::{channel::mpsc, executor::ThreadPool};
use mpsc::{UnboundedReceiver, UnboundedSender};
use num_format::Locale;
use std::path::{Path, PathBuf};

pub struct DPSDlg {
  state: AppState,
  threads: ThreadPool,
  locale: Locale,
  log_path: PathBuf,
  title: String,
  avatar: String,
  span: Span,
  channel: Channel,
  tally: Option<DPSTally>,
  visible: bool,
}

impl DPSDlg {
  pub fn new(state: AppState, threads: ThreadPool, locale: Locale) -> Self {
    let (tx, rx) = mpsc::unbounded();
    let cancel = Some(Cancel::default());
    let channel = Channel { tx, rx, cancel };

    // Default to the whole day for the search date/time span.
    let date = Local::now().naive_local().date();
    let begin = NaiveDateTime::new(date, NaiveTime::from_hms_opt(0, 0, 0).unwrap());
    let end = NaiveDateTime::new(date, NaiveTime::from_hms_opt(23, 59, 59).unwrap());
    let span = Span { begin, end };

    DPSDlg {
      state,
      threads,
      locale,
      log_path: PathBuf::default(),
      title: String::new(),
      avatar: String::new(),
      span,
      channel,
      tally: None,
      visible: false,
    }
  }

  pub fn open(&mut self, avatar: &str, path_buf: &Path) {
    if !avatar.is_empty() && !self.visible {
      path_buf.clone_into(&mut self.log_path);
      avatar.clone_into(&mut self.avatar);
      self.title = format!("⚔  Tally DPS ({avatar})");
      self.state.set_disabled(true);
      self.tally = None;
      self.visible = true;
    }
  }

  pub fn show(&mut self, ctx: &Context) {
    while let Ok(Some(tally)) = self.channel.rx.try_next() {
      // Update the date/time span and store the tally.
      self.span = tally.span.clone();
      self.tally = Some(tally);
      self.state.set_busy(false);
    }

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
          // Date/time entry.
          ui.horizontal(|ui| {
            const LABEL_COLOR: Color32 = Color32::from_rgb(154, 187, 154);
            let x_spacing = ui.spacing().item_spacing.x;

            ui.spacing_mut().item_spacing.x *= 0.5;
            ui.label(RichText::from("Begin").color(LABEL_COLOR));
            ui.spacing_mut().item_spacing.x = x_spacing;
            if let Some(date_time) = show_date_time(ui, &self.span.begin, "begin_date_picker") {
              self.span.begin = date_time;
              self.tally = None;
            }

            ui.separator();

            ui.spacing_mut().item_spacing.x *= 0.5;
            ui.label(RichText::from("End").color(LABEL_COLOR));
            ui.spacing_mut().item_spacing.x = x_spacing;
            if let Some(date_time) = show_date_time(ui, &self.span.end, "end_date_picker") {
              self.span.end = date_time;
              self.tally = None;
            }
          });

          ui.separator();

          if let Some(tally) = &self.tally {
            // Damage/DPS output.
            ui.horizontal(|ui| {
              Grid::new("dps_grid")
                .min_col_width((ui.available_width() - ui.spacing().item_spacing.x * 3.0) / 4.0)
                .show(ui, |ui| {
                  // Header.
                  const HEADER_COLOR: Color32 = Color32::from_rgb(229, 187, 123);
                  ui.label(RichText::from("Total Damage").color(HEADER_COLOR));
                  ui.label(RichText::from("Total DPS").color(HEADER_COLOR));
                  ui.label(RichText::from("Avatar DPS").color(HEADER_COLOR));
                  ui.label(RichText::from("Pet DPS").color(HEADER_COLOR));
                  ui.end_row();

                  // Total damage.
                  let total_damage = tally.avatar + tally.pet;
                  let text = format!("{total_damage}");
                  ui.label(text);

                  // Total DPS.
                  let val = total_damage as f64 / tally.secs as f64;
                  let text = f64_to_string!(val, 2, self.locale);
                  ui.label(text);

                  // Avatar DPS.
                  let val = tally.avatar as f64 / tally.secs as f64;
                  let text = f64_to_string!(val, 2, self.locale);
                  ui.label(text);

                  // Pet DPS.
                  let val = tally.pet as f64 / tally.secs as f64;
                  let text = f64_to_string!(val, 2, self.locale);
                  ui.label(text);
                });
            });

            ui.separator();
          }

          ui.horizontal(|ui| {
            if ui.button("Tally").clicked() {
              self.request_dps_tally(ctx);
            }

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

  fn request_dps_tally(&mut self, ctx: &Context) {
    self.tally = None;

    // Cancel any previous request.
    if let Some(mut cancel) = self.channel.cancel.take() {
      cancel.cancel();
    }

    let cancel = Cancel::default();
    self.channel.cancel = Some(cancel.clone());

    // Show the busy cursor.
    self.state.set_busy(true);

    // Setup the future.
    let tx = self.channel.tx.clone();
    let ctx = ctx.clone();
    let log_path = self.log_path.clone();
    let avatar = self.avatar.clone();
    let span = self.span.clone();
    let future = log_data::tally_dps(log_path, avatar, span, cancel);
    let future = async move {
      tx.unbounded_send(future.await).unwrap();
      ctx.request_repaint();
    };

    // Execute the future on a pooled thread.
    self.threads.spawn_ok(future);
  }

  fn close(&mut self) {
    if self.visible {
      if let Some(mut cancel) = self.channel.cancel.take() {
        // Cancel the tally request if it's still outstanding.
        cancel.cancel();
      }

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

fn show_date_time(ui: &mut Ui, date_time: &NaiveDateTime, id: &str) -> Option<NaiveDateTime> {
  let mut result = None;
  let x_spacing = ui.spacing().item_spacing.x;
  let x_interact = ui.spacing().interact_size.x;

  // Date.
  let mut date = date_time.date();
  let widget = DatePickerButton::new(&mut date)
    .id_source(id)
    .show_icon(false);
  ui.spacing_mut().item_spacing.x *= 0.5;
  if ui.add(widget).changed() {
    result = Some(NaiveDateTime::new(date, date_time.time()));
  }

  // Hour.
  let mut hour = date_time.hour();
  let widget = DragValue::new(&mut hour)
    .custom_formatter(|val, _| format!("{val:02}"))
    .range(0..=23)
    .speed(0.125);
  ui.spacing_mut().item_spacing.x = 1.0;
  ui.spacing_mut().interact_size.x = 23.0;
  if ui.add(widget).changed() {
    result = Some(date_time.with_hour(hour).unwrap());
  }
  ui.label(":");

  // Minute.
  let mut min = date_time.minute();
  let widget = DragValue::new(&mut min)
    .custom_formatter(|val, _| format!("{val:02}"))
    .range(0..=59)
    .speed(0.125);
  if ui.add(widget).changed() {
    result = Some(date_time.with_minute(min).unwrap());
  }
  ui.label(":");

  // Second.
  let mut sec = date_time.second();
  let widget = DragValue::new(&mut sec)
    .custom_formatter(|val, _| format!("{val:02}"))
    .range(0..=59)
    .speed(0.125);
  ui.spacing_mut().item_spacing.x = x_spacing;
  if ui.add(widget).changed() {
    result = Some(date_time.with_second(sec).unwrap());
  }
  ui.spacing_mut().interact_size.x = x_interact;

  result
}

struct Channel {
  tx: UnboundedSender<DPSTally>,
  rx: UnboundedReceiver<DPSTally>,
  cancel: Option<Cancel>,
}
