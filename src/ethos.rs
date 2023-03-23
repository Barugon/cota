use crate::util::{FORTNIGHT_SECS, HOUR_SECS};

#[allow(unused)]
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

pub const CABALISTS: [&str; 8] = [
  "Dolus", "Temna", "Nefario", "Nefas", "Avara", "Indigno", "Corpus", "Fastus",
];

// The count can be changed to use mem::variant_count when it stabilizes.
pub const TOWNS: [&str; Virtue::Spirituality as usize + 1] = [
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

/// Orbital periods and zone times.
pub const PLANETARY_ORBITS: [(i64, f64); CABALISTS.len()] = [
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
