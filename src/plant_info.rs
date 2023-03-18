use crate::util::{HOUR_SECS, NONE_ERR};
use chrono::{Duration, Local, NaiveDateTime};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Serialize, Deserialize, Debug, PartialEq, Eq)]
pub enum Seed {
  Low = 1,
  Med = 2,
  High = 3,
}

impl Seed {
  fn new(text: &str) -> Option<Self> {
    match text {
      "1" => Some(Seed::Low),
      "2" => Some(Seed::Med),
      "3" => Some(Seed::High),
      _ => None,
    }
  }
}

/// Parse the seeds CSV.
pub fn parse_seeds() -> Vec<(&'static str, Seed)> {
  const SEEDS: &str = include_str!("res/seeds.csv");
  let mut result = Vec::new();
  for line in SEEDS.lines() {
    let mut iter = line.split(',');
    let Some(seed_name) = iter.next() else { break };
    let Some(seed_type) = iter.next() else { break };
    result.push((seed_name, Seed::new(seed_type).expect(NONE_ERR)));
  }
  result
}

#[derive(Clone, Copy, Serialize, Deserialize, Debug, PartialEq, Eq)]
pub enum Environment {
  Greenhouse = 12 * HOUR_SECS as isize / 3,
  Outside = 24 * HOUR_SECS as isize / 3,
  Inside = 240 * HOUR_SECS as isize / 3,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Event {
  None,
  Water,
  Harvest,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Plant {
  description: String,
  date_time: NaiveDateTime,
  seed_name: String,
  seed_type: Seed,
  environment: Environment,
  events: [Option<bool>; 3],
}

impl Plant {
  pub fn new(
    description: String,
    date_time: NaiveDateTime,
    seed_name: String,
    seed_type: Seed,
    environment: Environment,
  ) -> Self {
    Self {
      description,
      date_time,
      seed_name,
      seed_type,
      environment,
      events: [Some(false); 3],
    }
  }

  pub fn description(&self) -> &str {
    &self.description
  }

  pub fn date_time(&self) -> NaiveDateTime {
    self.date_time
  }

  pub fn seed_name(&self) -> &str {
    &self.seed_name
  }

  pub fn environment(&self) -> Environment {
    self.environment
  }

  /// Get the current event.
  pub fn current_event(&self) -> Event {
    if self.events[2] == Some(true) {
      return Event::Harvest;
    }

    if self.events[0] == Some(true) || self.events[1] == Some(true) {
      return Event::Water;
    }

    Event::None
  }

  /// Get the next event and it's date/time.
  pub fn next_event(&self) -> (Event, NaiveDateTime) {
    let elapsed = (Local::now().naive_local() - self.date_time).num_seconds();
    let interval = self.seed_type as i64 * self.environment as i64;

    for count in 1..=self.events.len() {
      let timeout = interval * count as i64;
      if elapsed < timeout {
        let date_time = self.date_time + Duration::seconds(timeout);
        if count < 3 {
          return (Event::Water, date_time);
        } else {
          return (Event::Harvest, date_time);
        }
      }
    }

    (Event::None, Default::default())
  }

  /// Check for events.
  pub fn check(&mut self) -> bool {
    let elapsed = (Local::now().naive_local() - self.date_time).num_seconds();
    let interval = self.seed_type as i64 * self.environment as i64;

    // Check the last event first.
    for count in (0..self.events.len()).rev() {
      if elapsed > interval * (count as i64 + 1) {
        if self.events[count] == Some(false) {
          // Flag this event.
          self.events[count] = Some(true);

          // Clear previous events.
          for count in (0..count).rev() {
            self.events[count] = None;
          }

          // Return true to signal a new event.
          return true;
        }

        break;
      }
    }

    false
  }

  /// Reset any events.
  pub fn reset_events(&mut self) {
    for event in &mut self.events {
      if *event == Some(true) {
        *event = None;
      }
    }
  }
}
