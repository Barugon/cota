use crate::util::{Cancel, HOUR_SECS};
use chrono::{DateTime, TimeZone, Utc};
use eframe::{
  egui::{Context, Grid, Layout, RichText, Ui},
  emath::Align,
  epaint::Color32,
};
use futures::executor::ThreadPool;
use std::time::Duration;

pub struct Chronometer {
  threads: ThreadPool,
  timer_cancel: Option<Cancel>,
}

impl Chronometer {
  pub fn new(threads: ThreadPool) -> Self {
    Self {
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
    let now = Utc::now();

    ui.add_space(4.0);

    Grid::new("lunar_rifts_grid")
      .min_col_width((width - spacing.x * 2.0) / 3.0)
      .show(ui, |ui| {
        // Header.
        ui.label(RichText::from("Portal").color(HEADER_COLOR));
        ui.centered_and_justified(|ui| {
          ui.label(RichText::from("Phase").color(HEADER_COLOR));
        });
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
          ui.label(RichText::from("Status").color(HEADER_COLOR));
        });
        ui.end_row();

        const LUNAR_RIFTS: [(&str, &str); RIFT_COUNT] = [
          ("Blood River", "New Moon"),
          ("Solace Bridge", "Waxing Crescent"),
          ("Highvale", "First Quarter"),
          ("Brookside", "Waxing Gibbous"),
          ("Owl's Head", "Full Moon"),
          ("Westend", "Wanning Gibbous"),
          ("Brittany Graveyard", "Third Quarter"),
          ("Etceter", "Wanning Crescent"),
        ];

        // Rifts.
        let rift_countdowns = get_rift_countdowns(now);
        for idx in 0..RIFT_COUNT {
          let countdown = rift_countdowns[idx];
          let (name, phase) = LUNAR_RIFTS[idx];
          let (rift_color, color, status) = if countdown < 0 {
            const OPEN_RIFT_COLOR: Color32 = Color32::from_rgb(154, 229, 255);
            (
              OPEN_RIFT_COLOR,
              ACTIVE_PORTAL_COLOR,
              get_countdown_text("Closes: ", -countdown),
            )
          } else {
            const CLOSED_RIFT_COLOR: Color32 = Color32::from_rgb(102, 154, 180);
            (
              CLOSED_RIFT_COLOR,
              INACTIVE_PORTAL_COLOR,
              get_countdown_text("Opens: ", countdown),
            )
          };

          ui.label(RichText::from(name).color(rift_color));
          ui.centered_and_justified(|ui| {
            ui.label(RichText::from(phase).color(color));
          });
          ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            ui.label(RichText::from(status).color(color));
          });
          ui.end_row();
        }
      });

    ui.add_space(4.0);

    Grid::new("lost_vale_grid")
      .min_col_width((width - spacing.x) / 2.0)
      .show(ui, |ui| {
        const LOST_VALE: &str = "Lost Vale";

        let countdown = get_lost_vale_countdown(now);
        let (vale_color, status_color, status) = if countdown < 0 {
          const OPEN_VALE_COLOR: Color32 = Color32::from_rgb(187, 187, 255);
          (
            OPEN_VALE_COLOR,
            ACTIVE_PORTAL_COLOR,
            get_countdown_text("Closes: ", -countdown),
          )
        } else {
          const CLOSED_VALE_COLOR: Color32 = Color32::from_rgb(140, 140, 187);
          (
            CLOSED_VALE_COLOR,
            INACTIVE_PORTAL_COLOR,
            get_countdown_text("Opens: ", countdown),
          )
        };

        ui.label(RichText::from(LOST_VALE).color(vale_color));
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
          ui.label(RichText::from(status).color(status_color));
        });
        ui.end_row();
      });

    ui.add_space(4.0);
    ui.separator();
    ui.add_space(4.0);

    Grid::new("cabalists_grid")
      .min_col_width((width - spacing.x * 2.0) / 3.0)
      .show(ui, |ui| {
        // Header.
        ui.label(RichText::from("Cabalist").color(HEADER_COLOR));
        ui.centered_and_justified(|ui| {
          ui.label(RichText::from("Town (Devotional)").color(HEADER_COLOR));
        });
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
          ui.label(RichText::from("Remaining Time").color(HEADER_COLOR));
        });
        ui.end_row();

        const ETHOS_INDEX: usize = 6;
        const CABALISTS: [&str; CABALIST_COUNT] = [
          "Dolus", "Temna", "Nefario", "Nefas", "Avara", "Indigno", "Corpus", "Fastus",
        ];
        const TOWNS: [&str; TOWN_COUNT] = [
          "Kiln (Honor)",
          "Northwood (Sacrifice)",
          "Jaanaford (Justice)",
          "Point West (Valor)",
          "Brookside (Compassion)",
          "Etceter (Honesty)",
          "None (Ethos)",
          "Resolute (Courage)",
          "Ardoris (Love)",
          "Aerie (Truth)",
          "Eastmarch (Humility)",
          "Fortus End (Spirituality)",
        ];

        let sieges = get_sieges(now);
        for (index, siege) in sieges.into_iter().enumerate() {
          // Increment the town index for the next town.
          let next = (siege.town + 1) % 12;
          let next = format!("Next Town: {}", TOWNS[next]);

          let (cabalist_color, town_color) = if siege.town != ETHOS_INDEX {
            const ACTIVE_CABALIST_COLOR: Color32 = Color32::from_rgb(240, 140, 178);
            const ACTIVE_TOWN_COLOR: Color32 = Color32::from_gray(204);
            (ACTIVE_CABALIST_COLOR, ACTIVE_TOWN_COLOR)
          } else {
            const DORMANT_CABALIST_COLOR: Color32 = Color32::from_rgb(180, 120, 154);
            const DORMANT_TOWN_COLOR: Color32 = Color32::from_gray(128);
            (DORMANT_CABALIST_COLOR, DORMANT_TOWN_COLOR)
          };

          ui.label(RichText::from(CABALISTS[index]).color(cabalist_color))
            .on_hover_text_at_pointer(&next);
          ui.centered_and_justified(|ui| {
            ui.label(RichText::from(TOWNS[siege.town]).color(town_color))
              .on_hover_text_at_pointer(&next);
          });
          ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            ui.label(
              RichText::from(get_countdown_text(Default::default(), siege.remain_secs))
                .color(town_color),
            )
            .on_hover_text_at_pointer(next);
          });
          ui.end_row();
        }
      });
  }

  pub fn show_status(&mut self, ui: &mut Ui) {
    ui.centered_and_justified(|ui| {
      ui.label("The accuracy of this chronometer depends entirely on your system clock.\nFor best results, set your system clock to synchronize with Internet time.");
    });
  }

  pub fn start_timer(&mut self, ctx: &Context) {
    self.stop_timer();

    let ctx = ctx.clone();
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
}

fn get_countdown_text(prefix: &str, sec: i32) -> String {
  if sec >= 60 {
    let min = sec / 60;
    let sec = sec % 60;
    if min >= 60 {
      let hour = min / 60;
      let min = min % 60;
      return format!("{prefix}{hour:02}h {min:02}m {sec:02}s");
    }
    return format!("{prefix}{min:02}m {sec:02}s");
  }
  format!("{prefix}{sec:02}s")
}

/// SotA epoch (date/time of lunar cataclysm).
fn epoch() -> DateTime<Utc> {
  Utc.with_ymd_and_hms(1997, 9, 2, 0, 0, 0).unwrap() // LocalResult does not have expect.
}

const RIFT_COUNT: usize = 8;

// Get the number of seconds for each rift.
fn get_rift_countdowns(now: DateTime<Utc>) -> [i32; RIFT_COUNT] {
  const PHASE_SECS: i32 = 525;
  const CYCLE_SECS: i64 = 4200;

  // Get the number of seconds since epoch.
  let delta_secs = (now - epoch()).num_seconds();

  // Calculate the lunar phase from the delta. Each phase is 525 seconds and there are 8 phases, for a total of 4200
  // seconds per lunar cycle.
  let phase = (delta_secs % CYCLE_SECS) as i32;

  let mut rift = (phase / PHASE_SECS) as usize;
  let mut time = PHASE_SECS - phase % PHASE_SECS;
  let mut secs = [0; RIFT_COUNT];

  // Express the remaining time for the active rift as negative.
  secs[rift] = -time;

  for _ in 1..RIFT_COUNT {
    // Next rift.
    rift += 1;
    if rift >= RIFT_COUNT {
      rift = 0;
    }

    secs[rift] = time;

    // Add the minutes
    time += PHASE_SECS;
  }

  secs
}

/// Get the current Lost Vale countdown as seconds.
fn get_lost_vale_countdown(now: DateTime<Utc>) -> i32 {
  // Get the number of seconds since 2018/02/23 13:00:00 UTC (first sighting).
  let delta_secs = (now - Utc.with_ymd_and_hms(2018, 2, 23, 13, 0, 0).unwrap()).num_seconds(); // LocalResult does not have expect.

  // Calculate the time window using the original 28 hour duration (one in-game month).
  let win = delta_secs % (28 * HOUR_SECS);

  // Get the 11-11-6 hour segment within the time window (as of R57).
  let seg = win % (11 * HOUR_SECS);

  if seg < HOUR_SECS {
    // Lost vale is currently open.
    (seg - HOUR_SECS) as i32
  } else if win < (22 * HOUR_SECS) {
    // First two 11 hour segments.
    (11 * HOUR_SECS - seg) as i32
  } else {
    // Last 6 hour segment.
    (6 * HOUR_SECS - seg) as i32
  }
}

/// Number of seconds in a fortnight (two weeks, one in-game year).
const FORTNIGHT_SECS: i64 = HOUR_SECS * 24 * 14;
const CABALIST_COUNT: usize = 8;
const TOWN_COUNT: usize = 12;

#[derive(Default)]
struct Siege {
  town: usize,
  remain_secs: i32,
}

/// Calculate the town index and number of seconds remaining in a siege for each cabalist.
fn get_sieges(now: DateTime<Utc>) -> [Siege; CABALIST_COUNT] {
  // Orbital period of each planet in seconds.
  const DECEIT_SECS: i64 = HOUR_SECS * 19;
  const DESPISE_SECS: i64 = HOUR_SECS * 17;
  const DASTARD_SECS: i64 = HOUR_SECS * 13;
  const INJUSTICE_SECS: i64 = HOUR_SECS * 11;
  const PUNISHMENT_SECS: i64 = HOUR_SECS * 3;
  const DISHONOR_SECS: i64 = HOUR_SECS * 2;
  const CARNALITY_SECS: i64 = HOUR_SECS * 23;
  const VANITY_SECS: i64 = HOUR_SECS * 29;

  const CONSTELLATION_ZONE: f64 = 1.0 / TOWN_COUNT as f64;
  const CONSTELLATION_RATE: f64 = 1.0 / FORTNIGHT_SECS as f64;

  // Orbital periods and zone times.
  const PLANETARY_ORBITS: [(i64, f64); CABALIST_COUNT] = [
    (
      DECEIT_SECS,
      CONSTELLATION_ZONE / (1.0 / DECEIT_SECS as f64 - CONSTELLATION_RATE),
    ),
    (
      DESPISE_SECS,
      CONSTELLATION_ZONE / (1.0 / DESPISE_SECS as f64 - CONSTELLATION_RATE),
    ),
    (
      DASTARD_SECS,
      CONSTELLATION_ZONE / (1.0 / DASTARD_SECS as f64 - CONSTELLATION_RATE),
    ),
    (
      INJUSTICE_SECS,
      CONSTELLATION_ZONE / (1.0 / INJUSTICE_SECS as f64 - CONSTELLATION_RATE),
    ),
    (
      PUNISHMENT_SECS,
      CONSTELLATION_ZONE / (1.0 / PUNISHMENT_SECS as f64 - CONSTELLATION_RATE),
    ),
    (
      DISHONOR_SECS,
      CONSTELLATION_ZONE / (1.0 / DISHONOR_SECS as f64 - CONSTELLATION_RATE),
    ),
    (
      CARNALITY_SECS,
      CONSTELLATION_ZONE / (1.0 / CARNALITY_SECS as f64 - CONSTELLATION_RATE),
    ),
    (
      VANITY_SECS,
      CONSTELLATION_ZONE / (1.0 / VANITY_SECS as f64 - CONSTELLATION_RATE),
    ),
  ];

  PLANETARY_ORBITS.map(|(orbit_secs, zone_secs)| {
    // Get the number of seconds elapsed since epoch.
    let epoch_secs = (now - epoch()).num_seconds();

    // Current rotation of the constellations [0.0, 1.0).
    let constellation_orbit = (epoch_secs % FORTNIGHT_SECS) as f64 / FORTNIGHT_SECS as f64;

    // Current rotation of the planetary body [0.0, 1.0).
    let planet_orbit = (epoch_secs % orbit_secs) as f64 / orbit_secs as f64;

    // Planet position relative to the constellations [0.0, 12.0).
    let delta = planet_orbit - constellation_orbit;
    let delta = if delta < 0.0 { 1.0 + delta } else { delta };
    let zone_phase = TOWN_COUNT as f64 * delta;

    // The town index is the whole number.
    let town = zone_phase as usize;

    // Fractional part is the position within the zone.
    let remain_secs = (zone_secs - zone_phase.fract() * zone_secs).ceil() as i32;

    Siege { town, remain_secs }
  })
}
