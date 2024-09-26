use crate::{
  config::Config, dps_dlg::DPSDlg, log_data, log_dlg::LogDlg, notes_dlg::NotesDlg,
  search_dlg::SearchDlg, util,
};
use eframe::{
  egui::{ComboBox, Context, Layout, RichText, Ui},
  emath::Align,
  epaint::Color32,
};
use egui_extras::{Column, TableBuilder};
use futures::{channel::mpsc, executor::ThreadPool};
use log_data::StatsData;
use num_format::Locale;
use std::{
  collections::HashMap,
  mem,
  path::{Path, PathBuf},
};
use util::{AppState, Cancel, Search};

pub struct Stats {
  config: Config,
  resist_stats: HashMap<&'static str, (Resist, f64)>,

  // Threading.
  threads: ThreadPool,
  channel: Channel,

  // State.
  locale: Locale,
  log_path: PathBuf,
  state: AppState,

  // Collections.
  avatars: Vec<String>,
  dates: Vec<i64>,

  // Current selection.
  avatar: String,
  date: Option<i64>,

  // Stats.
  stats: StatsData,
  filter: StatsFilter,

  // Dialog windows.
  filter_dlg: SearchDlg,
  search_dlg: SearchDlg,
  notes_dlg: NotesDlg,
  log_dlg: LogDlg,
  dps_dlg: DPSDlg,

  // initialize on first view.
  init: bool,
}

impl Stats {
  pub fn new(
    log_path: PathBuf,
    threads: ThreadPool,
    config: Config,
    state: AppState,
    locale: Locale,
  ) -> Self {
    let resist_stats = HashMap::from([
      ("AirAttunement", (Resist::Air, 0.5)),
      ("AirResistance", (Resist::Air, 1.0)),
      ("ChaosAttunement", (Resist::Chaos, 0.5)),
      ("ChaosResistance", (Resist::Chaos, 1.0)),
      ("DeathAttunement", (Resist::Death, 0.5)),
      ("DeathResistance", (Resist::Death, 1.0)),
      ("EarthAttunement", (Resist::Earth, 0.5)),
      ("EarthResistance", (Resist::Earth, 1.0)),
      ("FireAttunement", (Resist::Fire, 0.5)),
      ("FireResistance", (Resist::Fire, 1.0)),
      ("LifeAttunement", (Resist::Life, 0.5)),
      ("LifeResistance", (Resist::Life, 1.0)),
      ("MoonAttunement", (Resist::Moon, 0.5)),
      ("MoonResistance", (Resist::Moon, 1.0)),
      ("SunAttunement", (Resist::Sun, 0.5)),
      ("SunResistance", (Resist::Sun, 1.0)),
      ("WaterAttunement", (Resist::Water, 0.5)),
      ("WaterResistance", (Resist::Water, 1.0)),
      ("MagicResistance", (Resist::Magic, 1.0)),
    ]);

    // Threading.
    let (tx, rx) = mpsc::unbounded();
    let channel = Channel {
      tx,
      rx,
      cancel_avatars: None,
      cancel_dates: None,
      cancel_stats: None,
      cancel_search: None,
    };

    // Collections
    let avatars = Vec::new();
    let dates = Vec::new();

    // Current selection.
    let avatar = String::new();
    let date = None;

    // Stats.
    let stats = StatsData::default();
    let filter = StatsFilter::None;

    // Dialog windows.
    let filter_dlg = SearchDlg::new(state.clone());
    let search_dlg = SearchDlg::new(state.clone());
    let notes_dlg = NotesDlg::new(state.clone());
    let log_dlg = LogDlg::new(state.clone());
    let dps_dlg = DPSDlg::new(state.clone(), threads.clone(), locale);

    Stats {
      config,
      resist_stats,
      threads,
      channel,
      locale,
      log_path,
      state,
      avatars,
      dates,
      avatar,
      date,
      stats,
      filter,
      filter_dlg,
      search_dlg,
      notes_dlg,
      log_dlg,
      dps_dlg,
      init: true,
    }
  }

  pub fn show(&mut self, ui: &mut Ui) {
    if mem::take(&mut self.init) {
      self.request_avatars(ui.ctx());
    }

    if !self.filter_dlg.show(ui.ctx()) {
      if let Some(search) = self.filter_dlg.take_search_term() {
        self.filter = StatsFilter::Search { search };
      }
    }

    if !self.search_dlg.show(ui.ctx()) {
      if let Some(search) = self.search_dlg.take_search_term() {
        self.search_logs(ui.ctx(), search);
      }
    }

    if !self.notes_dlg.show(ui.ctx()) {
      if let Some(text) = self.notes_dlg.take_text() {
        if !self.avatar.is_empty() {
          self.config.set_notes(&self.avatar, text);
        }
      }
    }

    self.log_dlg.show(ui.ctx());
    self.dps_dlg.show(ui.ctx());

    // Collect messages.
    while let Ok(Some(msg)) = self.channel.rx.try_next() {
      match msg {
        Message::Avatars(avatars) => {
          self.avatars = avatars;
          self.avatar.clear();

          // Determine the current avatar.
          if let Some(first) = self.avatars.first() {
            // Check if the avatar is in the configuration.
            if let Some(avatar) = self.config.get_stats_avatar() {
              if self.avatars.binary_search(&avatar).is_ok() {
                self.avatar = avatar;
              }
            }

            // If the avatar wasn't set then use the first avatar.
            if self.avatar.is_empty() {
              self.config.set_stats_avatar(first.clone());
              self.avatar.clone_from(first);
            }
          }

          // Get the dates for the current avatar.
          self.request_dates(ui.ctx());
        }
        Message::Dates(dates) => {
          self.dates = dates;
          self.date = self.dates.first().copied();
          self.request_stats(ui.ctx());
        }
        Message::Stats(stats) => {
          self.state.set_busy(false);
          self.stats = stats;
        }
        Message::Search(text, search) => {
          self.state.set_busy(false);
          self.log_dlg.set_text(text, search, ui.ctx());
        }
      }
    }

    // Tool bar.
    ui.horizontal(|ui| {
      // Avatar combo-box.
      ui.add_enabled_ui(!self.avatars.is_empty(), |ui| {
        let mut avatar_changed = false;
        ComboBox::from_id_salt("avatar_combo")
          .selected_text(&self.avatar)
          .width(250.0)
          .show_ui(ui, |ui| {
            for avatar in &self.avatars {
              if ui
                .selectable_label(self.avatar == *avatar, avatar)
                .clicked()
                && self.avatar != *avatar
              {
                self.config.set_stats_avatar(avatar.clone());
                self.avatar.clone_from(avatar);
                avatar_changed = true;
              }
            }
          });
        if avatar_changed {
          self.request_dates(ui.ctx());
        }
      });

      // Date combo-box.
      ui.add_enabled_ui(!self.dates.is_empty(), |ui| {
        let mut date_changed = false;
        ComboBox::from_id_salt("date_combo")
          .selected_text(util::timestamp_to_string(self.date))
          .show_ui(ui, |ui| {
            // This is here to keep the date text from wrapping when the scroll bar is visible.
            ui.set_min_width(137.0);
            for date in &self.dates {
              let date = Some(*date);
              let text = util::timestamp_to_string(date);
              if ui.selectable_label(self.date == date, text).clicked() && self.date != date {
                self.date = date;
                date_changed = true;
              }
            }
          });
        if date_changed {
          self.request_stats(ui.ctx());
        }
      });

      // Notes button.
      ui.add_enabled_ui(!self.avatar.is_empty(), |ui| {
        if ui.button("Notes").clicked() {
          let text = self.config.get_notes(&self.avatar);
          let text = text.unwrap_or_default();
          self.notes_dlg.open(&self.avatar, text);
        }
      });
    });

    // Stats.
    ui.add_enabled_ui(!self.stats.is_empty(), |ui| {
      const NAME_COLOR: Color32 = Color32::from_rgb(102, 154, 180);
      let spacing = ui.spacing().item_spacing;
      let row_size = util::text_size(ui) + spacing[1] * 2.0;
      let available_width = ui.available_width();
      TableBuilder::new(ui)
        .cell_layout(Layout::left_to_right(Align::Center))
        .striped(true)
        .column(Column::exact(available_width * 0.8 - spacing[0]))
        .column(Column::remainder())
        .header(row_size, |mut header| {
          const HEADER_COLOR: Color32 = Color32::from_rgb(229, 187, 123);
          header.col(|ui| {
            ui.label(RichText::from("Name").color(HEADER_COLOR));
          });
          header.col(|ui| {
            ui.label(RichText::from("Value").color(HEADER_COLOR));
          });
        })
        .body(|mut body| match &self.filter {
          StatsFilter::None => {
            for (name, value) in self.stats.iter() {
              body.row(row_size, |mut row| {
                row.col(|ui| {
                  ui.label(RichText::from(name).color(NAME_COLOR));
                });
                row.col(|ui| {
                  ui.label(f64_to_string!(value, 6, self.locale));
                });
              });
            }
          }
          StatsFilter::Resists => {
            // Collect and sum the resistances.
            let mut resist_values: HashMap<Resist, f64> = HashMap::new();
            for (name, value) in self.stats.iter() {
              if let Some((key, mul)) = self.resist_stats.get(name) {
                if let Some(resist) = resist_values.get_mut(key) {
                  *resist += value * mul;
                } else {
                  resist_values.insert(*key, value * mul);
                }
              }
            }

            // Add-in magic resistance.
            if let Some(magic) = resist_values.remove(&Resist::Magic) {
              for (key, resist) in &mut resist_values {
                // Chaos is not affected by magic resistance.
                if *key != Resist::Chaos {
                  *resist += magic;
                }
              }
            }

            const RESIST_KEYS: [(Resist, &str); 9] = [
              (Resist::Air, "Air"),
              (Resist::Chaos, "Chaos"),
              (Resist::Death, "Death"),
              (Resist::Earth, "Earth"),
              (Resist::Fire, "Fire"),
              (Resist::Life, "Life"),
              (Resist::Moon, "Moon"),
              (Resist::Sun, "Sun"),
              (Resist::Water, "Water"),
            ];

            for (key, name) in RESIST_KEYS {
              if let Some(value) = resist_values.get(&key) {
                let value = f64_to_string!(*value, 6, self.locale);
                body.row(row_size, |mut row| {
                  row.col(|ui| {
                    const RESIST_COLOR: Color32 = Color32::from_rgb(154, 120, 180);
                    ui.label(RichText::from(name).color(RESIST_COLOR));
                  });
                  row.col(|ui| {
                    ui.label(value);
                  });
                });
              }
            }
          }
          StatsFilter::Search { search: filter } => {
            for (name, value) in self.stats.iter() {
              if filter.find_in(name).is_some() {
                body.row(row_size, |mut row| {
                  row.col(|ui| {
                    ui.label(RichText::from(name).color(NAME_COLOR));
                  });
                  row.col(|ui| {
                    ui.label(f64_to_string!(value, 6, self.locale));
                  });
                });
              }
            }
          }
        });
    });
  }

  pub fn show_status(&self, ui: &mut Ui) {
    if self.avatar.is_empty() {
      return;
    }

    let date = util::timestamp_to_string(self.date);
    if date.is_empty() {
      return;
    }

    ui.centered_and_justified(|ui| {
      ui.label(match self.filter {
        StatsFilter::None => format!("Stats for {} from {}", self.avatar, date),
        StatsFilter::Resists => format!("Effective resists for {} from {}", self.avatar, date),
        StatsFilter::Search { search: _ } => {
          format!("Filtered stats for {} from {}", self.avatar, date)
        }
      });
    });
  }

  pub fn avatar(&self) -> &str {
    &self.avatar
  }

  pub fn stats(&self) -> &StatsData {
    &self.stats
  }

  pub fn filter(&self) -> &StatsFilter {
    &self.filter
  }

  pub fn set_filter(&mut self, filter: StatsFilter) {
    self.filter = filter;
  }

  pub fn show_filter_dlg(&mut self) {
    let title = "⚙  Filter Stats".into();
    self.filter_dlg.open(title);
  }

  pub fn show_search_dlg(&mut self) {
    let title = format!("🔍  Search Logs ({})", self.avatar);
    self.search_dlg.open(title);
  }

  pub fn show_dps_dlg(&mut self) {
    self.dps_dlg.open(&self.avatar, &self.log_path);
  }

  pub fn log_path(&self) -> &Path {
    &self.log_path
  }

  pub fn set_log_path(&mut self, ctx: &Context, log_path: PathBuf) {
    self.log_path = log_path;
    self.request_avatars(ctx);
  }

  pub fn reload(&mut self, ctx: &Context) {
    self.request_avatars(ctx);
  }

  pub fn on_exit(&mut self) {
    // Cancel all async operations on exit.
    let cancelers = [
      self.channel.cancel_avatars.take(),
      self.channel.cancel_dates.take(),
      self.channel.cancel_stats.take(),
      self.channel.cancel_search.take(),
    ];

    for mut cancel in cancelers.into_iter().flatten() {
      cancel.cancel();
    }
  }

  fn request_avatars(&mut self, ctx: &Context) {
    // Clear all these.
    self.avatars.clear();
    self.avatar.clear();
    self.dates.clear();
    self.date = None;
    self.stats = StatsData::default();

    // Cancel any previous request.
    if let Some(mut cancel) = self.channel.cancel_avatars.take() {
      cancel.cancel();
    }

    let cancel = Cancel::default();
    self.channel.cancel_avatars = Some(cancel.clone());

    // Show the busy cursor.
    self.state.set_busy(true);

    // Setup the future.
    let tx = self.channel.tx.clone();
    let ctx = ctx.clone();
    let future = log_data::get_avatars(self.log_path.clone(), cancel);
    let future = async move {
      let msg = Message::Avatars(future.await);
      tx.unbounded_send(msg).unwrap();
      ctx.request_repaint();
    };

    // Execute the future on a pooled thread.
    self.threads.spawn_ok(future);
  }

  fn request_dates(&mut self, ctx: &Context) {
    // Clear these.
    self.dates.clear();
    self.date = None;
    self.stats = StatsData::default();

    // Cancel any previous request.
    if let Some(mut cancel) = self.channel.cancel_dates.take() {
      cancel.cancel();
    }

    if !self.avatar.is_empty() {
      let cancel = Cancel::default();
      self.channel.cancel_dates = Some(cancel.clone());

      // Show the busy cursor.
      self.state.set_busy(true);

      // Setup the future.
      let log_path = self.log_path.clone();
      let avatar = self.avatar.clone();
      let threads = self.threads.clone();
      let future = log_data::get_stats_timestamps(log_path, avatar, cancel, Some(threads));
      let tx = self.channel.tx.clone();
      let ctx = ctx.clone();
      let future = async move {
        let msg = Message::Dates(future.await);
        tx.unbounded_send(msg).unwrap();
        ctx.request_repaint();
      };

      // Execute the future on a pooled thread.
      self.threads.spawn_ok(future);
      return;
    }

    self.state.set_busy(false);
  }

  fn request_stats(&mut self, ctx: &Context) {
    // Clear this.
    self.stats = StatsData::default();

    // Cancel any previous request.
    if let Some(mut cancel) = self.channel.cancel_stats.take() {
      cancel.cancel();
    }

    if let Some(date) = self.date {
      if !self.avatar.is_empty() {
        let cancel = Cancel::default();
        self.channel.cancel_stats = Some(cancel.clone());

        // Show the busy cursor.
        self.state.set_busy(true);

        // Setup the future.
        let tx = self.channel.tx.clone();
        let ctx = ctx.clone();
        let future = log_data::get_stats(self.log_path.clone(), self.avatar.clone(), date, cancel);
        let future = async move {
          let msg = Message::Stats(future.await);
          tx.unbounded_send(msg).unwrap();
          ctx.request_repaint();
        };

        // Execute the future on a pooled thread.
        self.threads.spawn_ok(future);
        return;
      }
    }

    self.state.set_busy(false);
  }

  fn search_logs(&mut self, ctx: &Context, search: Search) {
    if self.avatar.is_empty() {
      return;
    }

    let cancel = Cancel::default();
    self.channel.cancel_search = Some(cancel.clone());
    self.log_dlg.open(&self.avatar, cancel.clone());

    // Show the busy cursor.
    self.state.set_busy(true);

    // Setup the future.
    let tx = self.channel.tx.clone();
    let ctx = ctx.clone();
    let log_path = self.log_path.clone();
    let avatar = self.avatar.clone();
    let future = log_data::find_log_entries(log_path, avatar, search.clone(), cancel);
    let future = async move {
      let msg = Message::Search(future.await, search);
      tx.unbounded_send(msg).unwrap();
      ctx.request_repaint();
    };

    // Execute the future on a pooled thread.
    self.threads.spawn_ok(future);
  }
}

#[derive(Hash, Eq, PartialEq, Copy, Clone)]
enum Resist {
  Air,
  Chaos,
  Death,
  Earth,
  Fire,
  Life,
  Moon,
  Sun,
  Water,
  Magic,
}

#[derive(Clone)]
pub enum StatsFilter {
  /// Show all stats.
  None,

  /// Show effective resists.
  Resists,

  /// Filter the stats using Search.
  Search { search: Search },
}

impl StatsFilter {
  pub fn is_none(&self) -> bool {
    matches!(self, StatsFilter::None)
  }

  pub fn is_resists(&self) -> bool {
    matches!(self, StatsFilter::Resists)
  }
}

enum Message {
  Avatars(Vec<String>),
  Dates(Vec<i64>),
  Stats(StatsData),
  Search(String, Search),
}

struct Channel {
  tx: mpsc::UnboundedSender<Message>,
  rx: mpsc::UnboundedReceiver<Message>,
  cancel_avatars: Option<Cancel>,
  cancel_dates: Option<Cancel>,
  cancel_stats: Option<Cancel>,
  cancel_search: Option<Cancel>,
}
