use crate::util::{FORTNIGHT_SECS, HOUR_SECS};

pub struct Siege {
  virtue: Virtue,
  remain_secs: i32,
}

impl Siege {
  pub fn new(virtue: Virtue, remain_secs: i32) -> Self {
    Self {
      virtue,
      remain_secs,
    }
  }

  pub fn virtue(&self) -> Virtue {
    self.virtue
  }

  pub fn remain_secs(&self) -> i32 {
    self.remain_secs
  }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Virtue {
  Honor,
  Sacrifice,
  Justice,
  Valor,
  Compassion,
  Honesty,
  Ethos,
  Courage,
  Love,
  Truth,
  Humility,
  Spirituality,
}

pub const VIRTUES: &[Virtue] = &[
  Virtue::Honor,
  Virtue::Sacrifice,
  Virtue::Justice,
  Virtue::Valor,
  Virtue::Compassion,
  Virtue::Honesty,
  Virtue::Ethos,
  Virtue::Courage,
  Virtue::Love,
  Virtue::Truth,
  Virtue::Humility,
  Virtue::Spirituality,
];

pub const TOWNS: [&str; VIRTUES.len()] = [
  "Kiln",
  "Northwood",
  "Jaanaford",
  "Point West",
  "Brookside",
  "Etceter",
  "None",
  "Resolute",
  "Ardoris",
  "Aerie",
  "Eastmarch",
  "Fortus End",
];

pub const CABALISTS: &[&str] = &[
  "Dolus", "Temna", "Nefario", "Nefas", "Avara", "Indigno", "Corpus", "Fastus",
];

/// Orbital periods and zone times.
pub const PLANETARY_ORBITS: [(i64, f64); CABALISTS.len()] = [
  // Dolus.
  (
    DECEIT_SECS,
    CONSTELLATION_ZONE / (1.0 / DECEIT_SECS as f64 - CONSTELLATION_RATE),
  ),
  // Temna.
  (
    DESPISE_SECS,
    CONSTELLATION_ZONE / (1.0 / DESPISE_SECS as f64 - CONSTELLATION_RATE),
  ),
  // Nefario.
  (
    DASTARD_SECS,
    CONSTELLATION_ZONE / (1.0 / DASTARD_SECS as f64 - CONSTELLATION_RATE),
  ),
  // Nefas.
  (
    INJUSTICE_SECS,
    CONSTELLATION_ZONE / (1.0 / INJUSTICE_SECS as f64 - CONSTELLATION_RATE),
  ),
  // Avara.
  (
    PUNISHMENT_SECS,
    CONSTELLATION_ZONE / (1.0 / PUNISHMENT_SECS as f64 - CONSTELLATION_RATE),
  ),
  // Indigno.
  (
    DISHONOR_SECS,
    CONSTELLATION_ZONE / (1.0 / DISHONOR_SECS as f64 - CONSTELLATION_RATE),
  ),
  // Corpus.
  (
    CARNALITY_SECS,
    CONSTELLATION_ZONE / (1.0 / CARNALITY_SECS as f64 - CONSTELLATION_RATE),
  ),
  // Fastus.
  (
    VANITY_SECS,
    CONSTELLATION_ZONE / (1.0 / VANITY_SECS as f64 - CONSTELLATION_RATE),
  ),
];

// Orbital period of each planet in seconds.
const DECEIT_SECS: i64 = HOUR_SECS * 19;
const DESPISE_SECS: i64 = HOUR_SECS * 17;
const DASTARD_SECS: i64 = HOUR_SECS * 13;
const INJUSTICE_SECS: i64 = HOUR_SECS * 11;
const PUNISHMENT_SECS: i64 = HOUR_SECS * 3;
const DISHONOR_SECS: i64 = HOUR_SECS * 2;
const CARNALITY_SECS: i64 = HOUR_SECS * 23;
const VANITY_SECS: i64 = HOUR_SECS * 29;

const CONSTELLATION_ZONE: f64 = 1.0 / TOWNS.len() as f64;
const CONSTELLATION_RATE: f64 = 1.0 / FORTNIGHT_SECS as f64;
