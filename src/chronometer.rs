use crate::{
  ethos::{CABALISTS, PLANETARY_ORBITS, Siege, TOWNS, VIRTUES, Virtue},
  towns_dlg::TownsDlg,
  util::{self, AppState, Cancel, FORTNIGHT_SECS, HOUR_SECS},
};
use chrono::{DateTime, Local, TimeDelta, TimeZone, Utc};
use eframe::{
  egui::{Context, Grid, Layout, RichText, Ui},
  emath::Align,
  epaint::Color32,
};
use futures::executor::ThreadPool;
use std::time::Duration;

pub struct Chronometer {
  towns_dlg: TownsDlg,
  threads: ThreadPool,
  timer_cancel: Option<Cancel>,
}

impl Chronometer {
  pub fn new(threads: ThreadPool, state: AppState) -> Self {
    Self {
      towns_dlg: TownsDlg::new(state),
      threads,
      timer_cancel: None,
    }
  }

  pub fn show(&mut self, ui: &mut Ui) {
    const HEADER_COLOR: Color32 = Color32::from_rgb(229, 187, 123);
    const ACTIVE_PORTAL_COLOR: Color32 = Color32::from_gray(229);
    const INACTIVE_PORTAL_COLOR: Color32 = Color32::from_gray(128);

    let width = ui.available_width();
    let spacing = ui.spacing().item_spacing;
    let now = Local::now();
    let utc = now.to_utc();
    let sieges = get_sieges(utc);

    self.towns_dlg.show(ui.ctx(), &sieges);

    ui.add_space(4.0);
    Grid::new("lunar_rifts_grid")
      .min_col_width((width - spacing.x * 2.0) / 3.0)
      .show(ui, |ui| {
        // Header.
        ui.label(RichText::from("Portal").color(HEADER_COLOR));
        ui.centered_and_justified(|ui| {
          ui.label(RichText::from("Local Time").color(HEADER_COLOR));
        });
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
          ui.label(RichText::from("Countdown").color(HEADER_COLOR));
        });
        ui.end_row();

        // Rifts.
        let rift_countdowns = get_rift_countdowns(utc);
        for idx in 0..LUNAR_RIFTS.len() {
          let countdown = rift_countdowns[idx];
          let (rift_color, color, time, countdown) = if countdown < 0 {
            const OPEN_RIFT_COLOR: Color32 = Color32::from_rgb(154, 229, 255);
            let time = now + TimeDelta::seconds(-countdown as i64);
            let time = format!("Closes: {}", time.format("%H:%M:%S"));
            let countdown = util::get_countdown_text(-countdown);
            (OPEN_RIFT_COLOR, ACTIVE_PORTAL_COLOR, time, countdown)
          } else {
            const CLOSED_RIFT_COLOR: Color32 = Color32::from_rgb(102, 154, 180);
            let time = now + TimeDelta::seconds(countdown as i64);
            let time = format!("Opens: {}", time.format("%H:%M:%S"));
            let countdown = util::get_countdown_text(countdown);
            (CLOSED_RIFT_COLOR, INACTIVE_PORTAL_COLOR, time, countdown)
          };

          ui.label(RichText::from(LUNAR_RIFTS[idx]).color(rift_color));
          ui.centered_and_justified(|ui| {
            ui.label(RichText::from(time).color(color));
          });
          ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            ui.label(RichText::from(countdown).color(color));
          });
          ui.end_row();
        }
      });

    ui.scope(|ui| {
      ui.visuals_mut().widgets.noninteractive.bg_stroke.color = Color32::from_rgb(40, 40, 40);
      ui.separator();
    });

    Grid::new("lost_vale_grid")
      .min_col_width((width - spacing.x * 2.0) / 3.0)
      .show(ui, |ui| {
        const LOST_VALE: &str = "Lost Vale";

        let countdown = get_lost_vale_countdown(utc);
        let (vale_color, color, time, countdown) = if countdown < 0 {
          const OPEN_VALE_COLOR: Color32 = Color32::from_rgb(187, 187, 255);
          let time = now + TimeDelta::seconds(-countdown);
          let time = format!("Closes: {}", time.format("%H:%M"));
          let countdown = util::get_countdown_text(-countdown);
          (OPEN_VALE_COLOR, ACTIVE_PORTAL_COLOR, time, countdown)
        } else {
          const CLOSED_VALE_COLOR: Color32 = Color32::from_rgb(140, 140, 187);
          let time = now + TimeDelta::seconds(countdown);
          let time = format!("Opens: {}", time.format("%H:%M"));
          let countdown = util::get_countdown_text(countdown);
          (CLOSED_VALE_COLOR, INACTIVE_PORTAL_COLOR, time, countdown)
        };

        ui.label(RichText::from(LOST_VALE).color(vale_color));
        ui.centered_and_justified(|ui| {
          ui.label(RichText::from(time).color(color));
        });
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
          ui.label(RichText::from(countdown).color(color));
        });
        ui.end_row();
      });

    ui.scope(|ui| {
      ui.visuals_mut().widgets.noninteractive.bg_stroke.color = Color32::from_rgb(40, 40, 40);
      ui.separator();
    });

    Grid::new("lunar_grid")
      .min_col_width((width - spacing.x * 2.0) / 3.0)
      .show(ui, |ui| {
        let countdown = get_lunar_countdown(utc);
        let (color, time, countdown) = if countdown < 0 {
          let time = now + TimeDelta::seconds(-countdown);
          let time = format!("Moonrise: {}", time.format("%H:%M"));
          let status = util::get_countdown_text(-countdown);
          (INACTIVE_PORTAL_COLOR, time, status)
        } else {
          let time = now + TimeDelta::seconds(countdown);
          let time = format!("Moonset: {}", time.format("%H:%M"));
          let status = util::get_countdown_text(countdown);
          (ACTIVE_PORTAL_COLOR, time, status)
        };

        ui.label(String::new());
        ui.centered_and_justified(|ui| {
          ui.label(RichText::from(time).color(color));
        });
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
          ui.label(RichText::from(countdown).color(color));
        });
        ui.end_row();
      });

    ui.add_space(4.0);
    ui.separator();
    if ui.button("Towns").clicked() {
      self.towns_dlg.open();
    }
    ui.add_space(4.0);

    Grid::new("cabalists_grid")
      .min_col_width((width - spacing.x * 2.0) / 3.0)
      .show(ui, |ui| {
        // Header.
        ui.label(RichText::from("Cabalist").color(HEADER_COLOR));
        ui.centered_and_justified(|ui| {
          ui.label(RichText::from("Town (Virtue)").color(HEADER_COLOR));
        });
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
          ui.label(RichText::from("Remaining Time").color(HEADER_COLOR));
        });
        ui.end_row();

        let counts = count_cabalists(&sieges);
        for (index, siege) in sieges.into_iter().enumerate() {
          // Increment the town index for the next town.
          let next = (siege.virtue() as usize + 1) % 12;
          let next = format!("Next Town: {} ({:?})", TOWNS[next], VIRTUES[next]);

          // Get the colors.
          let (cabalist_color, town_color, remain_color) = if siege.virtue() != Virtue::Ethos {
            let town_color = match counts[siege.virtue() as usize] {
              0 => unreachable!(),
              1 => Color32::from_rgb(192, 164, 24),
              2 => Color32::from_rgb(208, 96, 32),
              _ => Color32::from_rgb(224, 48, 48),
            };
            (Color32::from_rgb(240, 140, 178), town_color, Color32::from_gray(204))
          } else {
            (
              Color32::from_rgb(144, 84, 107),
              Color32::from_gray(128),
              Color32::from_gray(128),
            )
          };

          // Cabalist.
          ui.label(RichText::from(CABALISTS[index]).color(cabalist_color))
            .on_hover_text(&next);

          // Town (virtue).
          ui.centered_and_justified(|ui| {
            let text = format!("{} ({:?})", TOWNS[siege.virtue() as usize], siege.virtue());
            ui.label(RichText::from(text).color(town_color)).on_hover_text(&next);
          });

          // Remaining time.
          ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            let text = util::get_countdown_text(siege.remain_secs());
            ui.label(RichText::from(text).color(remain_color)).on_hover_text(next);
          });
          ui.end_row();
        }
      });
  }

  pub fn show_status(&mut self, ui: &mut Ui) {
    ui.centered_and_justified(|ui| {
      const MSG: &str = concat!(
        "The accuracy of this chronometer depends entirely on your system clock.\n",
        "For best results, set your system clock to synchronize with Internet time."
      );
      ui.label(MSG);
    });
  }

  pub fn start_timer(&mut self, ctx: Context) {
    self.stop_timer();

    let cancel = Cancel::default();
    self.timer_cancel = Some(cancel.clone());

    self.threads.spawn_ok(async move {
      while !cancel.is_canceled() {
        // Request a repaint every quarter-second.
        std::thread::sleep(Duration::from_millis(250));
        ctx.request_repaint();
      }
    });
  }

  pub fn stop_timer(&mut self) {
    if let Some(mut timer_cancel) = self.timer_cancel.take() {
      timer_cancel.cancel();
    }
  }

  pub fn on_exit(&mut self) {
    self.stop_timer();
  }
}

const LUNAR_RIFTS: &[&str] = &[
  "Blood River",
  "Solace Bridge",
  "Highvale",
  "Brookside",
  "Owl's Head",
  "Westend",
  "Brittany Graveyard",
  "Etceter",
];

// Get the countdown (as seconds) for each rift.
fn get_rift_countdowns(now: DateTime<Utc>) -> [i64; LUNAR_RIFTS.len()] {
  const PHASE_SECS: i64 = 525;
  const CYCLE_SECS: i64 = 4200;

  // Get the number of seconds since epoch.
  let delta_secs = (now - util::get_epoch()).num_seconds();

  // Calculate the lunar phase from the delta. Each phase is 525 seconds and there are 8 phases, for a total of 4200
  // seconds per lunar cycle.
  let phase = delta_secs % CYCLE_SECS;

  let mut rift = (phase / PHASE_SECS) as usize;
  let mut time = PHASE_SECS - phase % PHASE_SECS;
  let mut secs = [0; LUNAR_RIFTS.len()];

  // Express the remaining time for the active rift as negative.
  secs[rift] = -time;

  for _ in 1..LUNAR_RIFTS.len() {
    // Next rift.
    rift += 1;
    if rift >= LUNAR_RIFTS.len() {
      rift = 0;
    }

    secs[rift] = time;

    // Add the minutes
    time += PHASE_SECS;
  }

  secs
}

/// Get the number of seconds until moonrise or moonset.
fn get_lunar_countdown(now: DateTime<Utc>) -> i64 {
  /// Number of seconds for one full orbit of the moon.
  const LUNAR_SECS: i64 = HOUR_SECS * 7;
  const LUNAR_QTR: i64 = LUNAR_SECS / 4;
  const LUNAR_3QTR: i64 = LUNAR_QTR * 3;

  // Get the number of seconds elapsed since epoch.
  let epoch_secs = (now - util::get_epoch()).num_seconds();

  // Current lunar position in seconds (zero is lunar high noon).
  let lunar_secs = epoch_secs % LUNAR_SECS;

  // Adjust so that [LUNAR_SECS / 2, 0) is moon up and [-LUNAR_SECS / 2, 0) is moon down.
  match lunar_secs {
    0..LUNAR_QTR => LUNAR_QTR - lunar_secs,
    LUNAR_QTR..LUNAR_3QTR => lunar_secs - LUNAR_3QTR,
    LUNAR_3QTR..LUNAR_SECS => (LUNAR_SECS + LUNAR_QTR) - lunar_secs,
    _ => unreachable!(),
  }
}

/// Get the current Lost Vale countdown as seconds.
fn get_lost_vale_countdown(now: DateTime<Utc>) -> i64 {
  // Get the number of seconds since 2018/02/23 13:00:00 UTC (first sighting).
  let delta_secs = (now - Utc.with_ymd_and_hms(2018, 2, 23, 13, 0, 0).unwrap()).num_seconds();

  // Calculate the time window using the original 28 hour duration (one in-game month).
  let win = delta_secs % (28 * HOUR_SECS);

  // Get the 11-11-6 hour segment within the time window (as of R57).
  let seg = win % (11 * HOUR_SECS);

  if seg < HOUR_SECS {
    // Lost vale is currently open.
    seg - HOUR_SECS
  } else if win < (22 * HOUR_SECS) {
    // First two 11 hour segments.
    11 * HOUR_SECS - seg
  } else {
    // Last 6 hour segment.
    6 * HOUR_SECS - seg
  }
}

/// Calculate the virtue/town and seconds remaining in a siege for each cabalist.
pub fn get_sieges(now: DateTime<Utc>) -> [Siege; CABALISTS.len()] {
  PLANETARY_ORBITS.map(|(orbit_secs, zone_secs)| {
    // Get the number of seconds elapsed since epoch.
    let epoch_secs = (now - util::get_epoch()).num_seconds();

    // Current rotational position of the constellations [0.0, 1.0).
    let constellation_orbit = (epoch_secs % FORTNIGHT_SECS) as f64 / FORTNIGHT_SECS as f64;

    // Current rotational position of the planetary body [0.0, 1.0).
    let planet_orbit = (epoch_secs % orbit_secs) as f64 / orbit_secs as f64;

    // Planet position relative to the constellations [0.0, 12.0).
    let delta = planet_orbit - constellation_orbit;
    let delta = if delta < 0.0 { 1.0 + delta } else { delta };
    let zone_phase = TOWNS.len() as f64 * delta;

    // The virtue/town is the whole number.
    let virtue = VIRTUES[zone_phase as usize];

    // Fractional part is the position within the zone.
    let remain_secs = (zone_secs - zone_phase.fract() * zone_secs).ceil() as i64;

    Siege::new(virtue, remain_secs)
  })
}

/// Get the number of cabalists for each siege.
fn count_cabalists(sieges: &[Siege; CABALISTS.len()]) -> [u32; VIRTUES.len()] {
  let mut counts: [u32; VIRTUES.len()] = Default::default();
  for siege in sieges {
    counts[siege.virtue() as usize] += 1;
  }
  counts
}
