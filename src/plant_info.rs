#![allow(unused)]
use std::time::Duration;

use crate::util::{HOUR_SECS, NONE_ERR};
use chrono::{DateTime, Local, TimeDelta};

#[derive(Clone, Copy)]
pub enum SeedType {
  Low = 1,
  Med = 2,
  High = 3,
}

impl SeedType {
  fn new(text: &str) -> Option<Self> {
    match text {
      "1" => Some(SeedType::Low),
      "2" => Some(SeedType::Med),
      "3" => Some(SeedType::High),
      _ => None,
    }
  }
}

/// Parse the seeds CSV.
pub fn parse_seeds() -> Vec<(&'static str, SeedType)> {
  const SEEDS: &str = include_str!("res/seeds.csv");
  let mut result = Vec::new();
  for line in SEEDS.lines() {
    let mut iter = line.split(',');
    let Some(seed_name) = iter.next() else { break };
    let Some(seed_type) = iter.next() else { break };
    result.push((seed_name, SeedType::new(seed_type).expect(NONE_ERR)));
  }
  result
}

#[derive(Clone, Copy)]
pub enum Environment {
  Greenhouse = 12 * HOUR_SECS as isize / 3,
  Outside = 24 * HOUR_SECS as isize / 3,
  Inside = 240 * HOUR_SECS as isize / 3,
}

#[derive(Clone, Copy)]
pub enum Event {
  None,
  Water,
  Harvest,
}

pub struct PlantInfo {
  description: String,
  date_time: DateTime<Local>,
  seed_name: &'static str,
  seed_type: SeedType,
  environment: Environment,
  event: [bool; 3],
}

impl PlantInfo {
  pub fn new(
    description: String,
    date_time: DateTime<Local>,
    seed_name: &'static str,
    seed_type: SeedType,
    environment: Environment,
  ) -> Self {
    Self {
      description,
      date_time,
      seed_name,
      seed_type,
      environment,
      event: Default::default(),
    }
  }

  pub fn description(&self) -> &str {
    &self.description
  }

  pub fn date_time(&self) -> DateTime<Local> {
    self.date_time
  }

  pub fn seed_name(&self) -> &str {
    self.seed_name
  }

  pub fn seed_type(&self) -> SeedType {
    self.seed_type
  }

  pub fn environment(&self) -> Environment {
    self.environment
  }

  /// Get the current event.
  pub fn current_event(&self) -> Event {
    if self.event[2] {
      return Event::Harvest;
    }

    if self.event[0] || self.event[1] {
      return Event::Water;
    }

    Event::None
  }

  /// Get the next event and it's date/time.
  pub fn next_event(&self) -> (Event, DateTime<Local>) {
    let elapsed = (Local::now() - self.date_time).num_seconds();
    let interval = self.seed_type as i64 * self.environment as i64;

    for count in 0..self.event.len() {
      let timeout = interval * count as i64;
      if elapsed < timeout {
        let date_time = self.date_time + TimeDelta::seconds(timeout);
        if count < 2 {
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
    let elapsed = (Local::now() - self.date_time).num_seconds();
    let interval = self.seed_type as i64 * self.environment as i64;

    // Check the last event first.
    for count in (0..self.event.len()).rev() {
      if elapsed > interval * count as i64 {
        if !self.event[count] {
          // Flag this event.
          self.event[count] = true;

          // Clear previous events.
          for count in (0..count).rev() {
            self.event[count] = false;
          }

          // Return true to signal a new event.
          return true;
        }

        break;
      }
    }

    false
  }
}
