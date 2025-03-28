use crate::util::{self, Cancel, Search};
use chrono::{DateTime, NaiveDate, NaiveDateTime, NaiveTime};
use futures::{StreamExt, channel::mpsc, executor::ThreadPool, future};
use regex::Regex;
use std::{
  collections::HashSet,
  fs,
  path::{Path, PathBuf},
  str::SplitWhitespace,
};

/// Get separate date/time and text portions of a log entry.
pub fn get_log_datetime_and_text(line: &str) -> (&str, &str) {
  if let Some(datetime) = get_log_datetime(line) {
    let text = &line[datetime.len()..];

    // Check if a chat timestamp was output.
    let trimmed = text.trim_start();
    if let Some(time) = get_log_datetime(trimmed) {
      return (datetime, &trimmed[time.len()..]);
    }

    return (datetime, text);
  }

  (Default::default(), line)
}

pub struct StatsIter<'a> {
  iter: SplitWhitespace<'a>,
}

impl StatsIter<'_> {
  fn new(text: &str) -> StatsIter {
    StatsIter {
      iter: text.split_whitespace(),
    }
  }
}

impl<'a> Iterator for StatsIter<'a> {
  type Item = (&'a str, f64);

  fn next(&mut self) -> Option<Self::Item> {
    // We're expecting "name: value" pairs.
    let name = self.iter.next()?.strip_suffix(':')?;
    let value = util::replace_decimal(self.iter.next()?).parse().ok()?;
    Some((name, value))
  }
}

#[derive(Default)]
pub struct StatsData {
  text: String,
}

impl StatsData {
  fn new(text: String) -> StatsData {
    StatsData { text }
  }

  pub fn is_empty(&self) -> bool {
    self.text.is_empty()
  }

  pub fn iter(&self) -> StatsIter<'_> {
    StatsIter::new(&self.text)
  }
}

const FILENAME_START: &str = "SotAChatLog";
const STATS_KEY: &str = " AdventurerLevel: ";
const ADV_EXP_KEY: &str = " Adventurer Experience: ";
const LOG_SEARCH_LIMIT: usize = 256 * 1024;

/// Get a vector of avatar names from the log file names.
pub async fn get_avatars(log_path: PathBuf, cancel: Cancel) -> Vec<String> {
  let filenames = get_log_filenames(&log_path, None, None);
  let mut name_set = HashSet::new();

  for filename in &filenames {
    if cancel.is_canceled() {
      return Vec::new();
    }

    let filename = &filename[FILENAME_START.len() + 1..];
    if let Some(pos) = filename.rfind('_') {
      name_set.insert(&filename[..pos]);
    }
  }

  let mut avatars = Vec::with_capacity(name_set.len());
  for name in name_set {
    if cancel.is_canceled() {
      return Vec::new();
    }

    avatars.push(String::from(name));
  }

  if cancel.is_canceled() {
    return Vec::new();
  }

  // Sort the avatars.
  avatars.sort_unstable();
  avatars
}

/// Get a vector of timestamps where `/stats` was used for the specified avatar.
pub async fn get_stats_timestamps(
  log_path: PathBuf,
  avatar: String,
  cancel: Cancel,
  threads: Option<ThreadPool>,
) -> Vec<i64> {
  // Collect the futures, one for each matching log file.
  let futures = {
    let filenames = get_log_filenames(&log_path, Some(&avatar), None);
    let mut futures = Vec::with_capacity(filenames.len());

    for filename in filenames {
      if cancel.is_canceled() {
        return Vec::new();
      }

      let path = log_path.join(filename.as_str());
      let cancel = cancel.clone();
      futures.push(async move {
        let Some(date) = get_log_file_date(&path) else {
          return Vec::new();
        };
        let text = ok!(fs::read_to_string(&path), Vec::new());
        let mut timestamps = Vec::new();

        for line in text.lines() {
          if cancel.is_canceled() {
            return Vec::new();
          }

          if let Some((ts, _)) = get_stats_timestamp_and_text(line, date) {
            timestamps.push(ts);
          }
        }

        timestamps
      });
    }

    futures
  };

  let results = if let Some(threads) = threads {
    // Process each future on a pooled thread.
    let (tx, rx) = mpsc::unbounded();
    for future in futures {
      let tx = tx.clone();
      threads.spawn_ok(async move {
        let result = future.await;
        tx.unbounded_send(result).unwrap();
      });
    }
    drop(tx);
    rx.collect().await
  } else {
    // Collect the results directly.
    future::join_all(futures).await
  };

  if cancel.is_canceled() {
    return Vec::new();
  }

  // Flatten the results.
  let mut timestamps: Vec<i64> = results.into_iter().flat_map(|v| v.into_iter()).collect();

  // Sort the timestamps so that the most recent is first.
  timestamps.sort_unstable_by(|a, b| b.cmp(a));
  timestamps
}

/// Get the stats for the specified avatar and timestamp.
pub async fn get_stats(log_path: PathBuf, avatar: String, timestamp: i64, cancel: Cancel) -> StatsData {
  if !avatar.is_empty() {
    let filenames = get_log_filenames(&log_path, Some(&avatar), Some(timestamp));

    // There will actually only be one file with the specific avatar name and date.
    for filename in filenames {
      let path = log_path.join(filename.as_str());
      if let Some(date) = get_log_file_date(&path) {
        if let Ok(text) = fs::read_to_string(path) {
          // Find the line with the specific date/time.
          for line in text.lines() {
            if cancel.is_canceled() {
              return StatsData::default();
            }

            if let Some(stats) = get_stats_text(line, timestamp, date) {
              // Include subsequent lines that do not start with a square bracket.
              let pos = util::offset(&text, stats).unwrap();
              let sub = &text[pos + stats.len()..];
              for line in sub.lines() {
                if line.starts_with('[') {
                  let stats = text[pos..util::offset(&text, line).unwrap()].trim();
                  return StatsData::new(stats.into());
                }
              }

              // EOF was reached.
              let stats = text[pos..].trim();
              return StatsData::new(stats.into());
            }
          }
        }
      }
    }
  }

  StatsData::default()
}

/// Get the latest adventurer experience from `/xp`.
pub async fn get_adv_exp(log_path: PathBuf, avatar: String, cancel: Cancel) -> Option<i64> {
  // Work on files from newest to oldest.
  let filenames = {
    let mut filenames = get_log_filenames(&log_path, Some(&avatar), None);
    filenames.sort_unstable_by(|a, b| b.cmp(a));
    filenames
  };

  for filename in filenames {
    if cancel.is_canceled() {
      break;
    }

    let path = log_path.join(filename);
    if let Ok(text) = fs::read_to_string(path) {
      if text.is_empty() {
        continue;
      }

      // Search from the latest entry.
      for line in text.lines().rev() {
        let exp = get_adv_xp(line);
        if exp.is_some() {
          return exp;
        }
      }
    }
  }

  None
}

/// Find log entries matching the provided search term.
pub async fn find_log_entries(log_path: PathBuf, avatar: String, search: Search, cancel: Cancel) -> String {
  let filenames = {
    let mut filenames = get_log_filenames(&log_path, Some(&avatar), None);

    // Work on files from newest to oldest.
    filenames.sort_unstable_by(|a, b| b.cmp(a));
    filenames
  };

  let mut result = String::new();
  for filename in filenames {
    if cancel.is_canceled() {
      return String::new();
    }

    let path = log_path.join(filename);
    if let Ok(text) = fs::read_to_string(path) {
      if text.is_empty() || !verify_log_text(&text) {
        continue;
      }

      // Iterate through the lines in reverse order (newest to oldest).
      for line in text.lines().rev() {
        if cancel.is_canceled() {
          return String::new();
        }

        // Split the date and text.
        let (datetime, text) = get_log_datetime_and_text(line);

        // Search the text portion.
        if search.find_in(text).is_some() {
          result.push_str(datetime);
          result.push_str(text);
          result.push('\n');
          if result.len() >= LOG_SEARCH_LIMIT {
            return result;
          }
        }
      }
    }
  }

  result
}

#[derive(Clone)]
pub struct Span {
  pub begin: NaiveDateTime,
  pub end: NaiveDateTime,
}

pub struct DPSTally {
  pub span: Span,
  pub avatar: u64,
  pub pet: u64,
  pub secs: u64,
}

impl DPSTally {
  fn new(span: Span) -> Self {
    Self {
      span,
      avatar: 0,
      pet: 0,
      secs: 0,
    }
  }
}

pub async fn tally_dps(log_path: PathBuf, avatar: String, span: Span, cancel: Cancel) -> DPSTally {
  let filenames = {
    let begin = span.begin.date();
    let end = span.end.date();

    // Filter the filenames to the date range.
    let filenames: Vec<String> = get_log_filenames(&log_path, Some(&avatar), None)
      .into_iter()
      .filter(|filename| {
        let path = Path::new(filename);
        if let Some(date) = get_log_file_date(path) {
          return date >= begin && date <= end;
        }
        false
      })
      .collect();
    filenames
  };

  let mut dps_tally = DPSTally::new(span.clone());
  if cancel.is_canceled() {
    return dps_tally;
  }

  // Use regular expressions for the searches.
  let avatar_search = format!("^ {avatar} attacks .+ and hits, dealing [0-9]+");
  let avatar_search = ok!(Regex::new(&avatar_search), dps_tally);
  let pet_search = format!("<{avatar}> attacks .+ and hits, dealing [0-9]+");
  let pet_search = ok!(Regex::new(&pet_search), dps_tally);

  // Range for checking log entry date/time.
  let begin_ts = span.begin.and_utc().timestamp();
  let end_ts = span.end.and_utc().timestamp();
  let range = if end_ts >= begin_ts {
    begin_ts..=end_ts
  } else {
    end_ts..=begin_ts
  };

  // Actual damage start and end timestamps.
  let mut dmg_start_ts = None;
  let mut dmg_end_ts = None;

  for filename in filenames {
    if cancel.is_canceled() {
      return DPSTally::new(span.clone());
    }

    // Read the log file.
    let path = log_path.join(filename);
    let file_date = get_log_file_date(&path).unwrap();
    if let Ok(text) = fs::read_to_string(path) {
      // Search for attack lines.
      for line in text.lines() {
        let (datetime, text) = get_log_datetime_and_text(line);
        if datetime.is_empty() {
          continue;
        }

        let Some(ts) = log_datetime_to_timestamp(datetime, file_date) else {
          continue;
        };

        if !range.contains(&ts) {
          continue;
        }

        if let Some(found) = avatar_search.find(text) {
          // The search term ends just past the damage value.
          if let Some(digits) = text[found.range()].split_whitespace().next_back() {
            if let Ok(value) = digits.parse::<u64>() {
              if dmg_start_ts.is_none() {
                dmg_start_ts = Some(ts);
              }
              dmg_end_ts = Some(ts);
              dps_tally.avatar += value;
            }
          }
        } else if let Some(found) = pet_search.find(text) {
          if let Some(digits) = text[found.range()].split_whitespace().next_back() {
            if let Ok(value) = digits.parse::<u64>() {
              if dmg_start_ts.is_none() {
                dmg_start_ts = Some(ts);
              }
              dmg_end_ts = Some(ts);
              dps_tally.pet += value;
            }
          }
        }
      }
    }
  }

  if let Some(start_ts) = dmg_start_ts {
    if let Some(begin) = DateTime::from_timestamp(start_ts, 0) {
      // Update the begin data/time.
      dps_tally.span.begin = begin.naive_utc();
    }
    if let Some(end_ts) = dmg_end_ts {
      if let Some(end) = DateTime::from_timestamp(end_ts, 0) {
        // Update the end data/time.
        dps_tally.span.end = end.naive_utc();
      }
      dps_tally.secs = 0.max(end_ts - start_ts) as u64;
    }
  }

  dps_tally.secs += 1;
  dps_tally
}

fn get_log_filenames(log_path: &Path, avatar: Option<&str>, timestamp: Option<i64>) -> Vec<String> {
  let mut filenames = Vec::new();
  let entries = ok!(log_path.read_dir(), filenames);

  // The name text is either a specific avatar or, if not specified, a regex wildcard.
  let name = avatar.unwrap_or(".+");

  // The date text is either a specific date or, if not specified, regex to match the date.
  let date = if let Some(timestamp) = timestamp {
    timestamp_to_file_date(timestamp)
  } else {
    String::from(r"\d{4}-\d{2}-\d{2}")
  };

  let regex = ok!(Regex::new(&format!("^{FILENAME_START}_{name}_{date}.txt$")), filenames);

  for entry in entries.flatten() {
    if let Some(filename) = entry.file_name().to_str() {
      if regex.is_match(filename) {
        filenames.push(filename.to_string());
      }
    }
  }

  filenames
}

/// Make sure the text contains at least one date/time.
fn verify_log_text(text: &str) -> bool {
  for line in text.lines() {
    if get_log_datetime(line).is_some() {
      return true;
    }
  }
  false
}

/// Convert a SotA log date & time into a timestamp. Since the dates are localized, we don't know
/// if day or month come first, so we use the date from the filename, which is always YYYY-MM-DD.
fn log_datetime_to_timestamp(text: &str, date: NaiveDate) -> Option<i64> {
  let text = text.trim_start_matches('[').trim_end_matches(']');
  let mut iter = text.split_whitespace();
  let _date = iter.next()?;
  let time = iter.next()?;
  let ap = iter.next();

  // Parse the hour and adjust for AM/PM.
  let mut iter = time.split(':');
  let hour = {
    let mut hour = iter.next()?.parse().ok()?;
    if let Some(ap) = ap {
      if let Some(ch) = ap.chars().next() {
        if ch == 'P' || ch == 'p' {
          // 12pm stays 12.
          if hour < 12 {
            // Add 12 to the hour.
            hour += 12;
          }
        } else if hour == 12 {
          // 12am becomes 0.
          hour = 0;
        }
      }
    }
    hour
  };

  let minute = iter.next()?.parse().ok()?;
  let second = iter.next()?.parse().ok()?;

  Some(
    NaiveDateTime::new(date, NaiveTime::from_hms_opt(hour, minute, second)?)
      .and_utc()
      .timestamp(),
  )
}

/// Convert a timestamp into a log filename date string.
fn timestamp_to_file_date(timestamp: i64) -> String {
  let Some(datetime) = DateTime::from_timestamp(timestamp, 0) else {
    return String::default();
  };
  datetime.format("%Y-%m-%d").to_string()
}

/// Get a NaiveDate from a log filename.
fn get_log_file_date(path: &Path) -> Option<NaiveDate> {
  let filename = path.file_stem()?.to_str()?;
  let pos = filename.rfind('_')?;
  let text = &filename[pos + 1..];
  NaiveDate::parse_from_str(text, "%Y-%m-%d").ok()
}

/// Get the date/time portion of a log entry.
fn get_log_datetime(line: &str) -> Option<&str> {
  if !line.starts_with('[') {
    return None;
  }

  let pos = line.find(']')?;
  Some(&line[0..=pos])
}

/// Get the log entry date/time as a timestamp and the log text if it's a `/stats` entry.
fn get_stats_timestamp_and_text(line: &str, file_date: NaiveDate) -> Option<(i64, &str)> {
  let (datetime, text) = get_log_datetime_and_text(line);
  if !datetime.is_empty() && text.starts_with(STATS_KEY) {
    let timestamp = log_datetime_to_timestamp(datetime, file_date)?;
    return Some((timestamp, text));
  }

  None
}

/// Get the log entry text if it's `/stats` and the date/time matches.
fn get_stats_text(line: &str, timestamp: i64, file_date: NaiveDate) -> Option<&str> {
  let (line_timestamp, text) = get_stats_timestamp_and_text(line, file_date)?;
  if line_timestamp == timestamp {
    return Some(text);
  }

  None
}

fn get_adv_xp(line: &str) -> Option<i64> {
  let (_, text) = get_log_datetime_and_text(line);
  let text = text.strip_prefix(ADV_EXP_KEY)?;
  util::remove_separators(text).parse().ok()
}
