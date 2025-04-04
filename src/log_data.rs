use crate::util::{self, Cancel, Search};
use chrono::{DateTime, NaiveDate, NaiveDateTime, NaiveTime};
use eframe::egui::{
  Color32, FontId, TextFormat,
  text::{LayoutJob, LayoutSection},
};
use futures::{StreamExt, channel::mpsc, executor::ThreadPool};
use regex::Regex;
use std::{
  collections::HashSet,
  fs,
  path::{Path, PathBuf},
  str::SplitWhitespace,
  sync::Arc,
};

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

#[derive(Clone, Default)]
pub struct StatsData {
  text: Arc<str>,
}

impl StatsData {
  fn new(text: String) -> StatsData {
    StatsData { text: text.into() }
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
pub async fn get_stats_timestamps(log_path: PathBuf, avatar: String, cancel: Cancel, threads: ThreadPool) -> Vec<i64> {
  let filenames = get_log_filenames(&log_path, Some(&avatar), None);
  let (tx, rx) = mpsc::unbounded();
  for filename in filenames {
    if cancel.is_canceled() {
      break;
    }

    // Process the log file on a pooled thread.
    let path = log_path.join(filename.as_ref());
    let cancel = cancel.clone();
    let tx = tx.clone();
    threads.spawn_ok(async move {
      let date = get_log_file_date(&path).unwrap();
      let text = ok!(fs::read_to_string(&path));
      let mut timestamps = Vec::new();

      for line in text.lines() {
        if cancel.is_canceled() {
          return;
        }

        if let Some((ts, _)) = get_stats_timestamp_and_text(line, date) {
          timestamps.push(ts);
        }
      }

      tx.unbounded_send(timestamps).unwrap();
    });
  }

  // Drop the sender to break the pipe when all tasks are done.
  drop(tx);

  // Collect the results.
  let results: Vec<Vec<i64>> = rx.collect().await;
  if cancel.is_canceled() {
    return Vec::new();
  }

  // Flatten the results.
  let mut timestamps: Vec<i64> = results.into_iter().flat_map(|v| v.into_iter()).collect();
  if cancel.is_canceled() {
    return Vec::new();
  }

  // Sort the timestamps so that the most recent is first.
  timestamps.sort_unstable_by(|a, b| b.cmp(a));
  timestamps
}

/// Get the stats for the specified avatar and timestamp.
pub async fn get_stats(log_path: PathBuf, avatar: String, timestamp: i64, cancel: Cancel) -> StatsData {
  if avatar.is_empty() {
    return StatsData::default();
  }

  let filenames = get_log_filenames(&log_path, Some(&avatar), Some(timestamp));

  // There will actually only be one file with the specific avatar name and date.
  for filename in filenames {
    let path = log_path.join(filename.as_ref());
    let Some(date) = get_log_file_date(&path) else {
      continue;
    };

    let Ok(text) = fs::read_to_string(path) else {
      continue;
    };

    // Find the line with the specific date/time.
    for line in text.lines() {
      if cancel.is_canceled() {
        return StatsData::default();
      }

      let Some(stats) = get_stats_text(line, timestamp, date) else {
        continue;
      };

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

  StatsData::default()
}

/// Get the latest adventurer experience from `/xp`.
pub async fn get_adv_exp(log_path: PathBuf, avatar: String, cancel: Cancel) -> Option<i64> {
  let filenames = get_sorted_log_filenames(&log_path, Some(&avatar));
  for filename in filenames {
    if cancel.is_canceled() {
      break;
    }

    let path = log_path.join(filename.as_ref());
    let Ok(text) = fs::read_to_string(path) else {
      continue;
    };

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

  None
}

/// Find log entries matching the provided search term.
pub async fn find_log_entries(
  log_path: PathBuf,
  avatar: String,
  search: Search,
  font: FontId,
  color: Color32,
  cancel: Cancel,
) -> LayoutJob {
  let filenames = get_sorted_log_filenames(&log_path, Some(&avatar));
  let format_normal = TextFormat::simple(font.clone(), color);
  let format_datetime = TextFormat::simple(font.clone(), Color32::from_rgb(180, 154, 102));
  let format_match = TextFormat::simple(font.clone(), Color32::from_rgb(102, 154, 180));

  let mut layout = LayoutJob {
    text: String::new(),
    sections: Vec::new(),
    break_on_newline: true,
    ..Default::default()
  };

  for filename in filenames {
    if cancel.is_canceled() {
      return LayoutJob::default();
    }

    let path = log_path.join(filename.as_ref());
    let Ok(text) = fs::read_to_string(path) else {
      continue;
    };

    if text.is_empty() || !verify_log_text(&text) {
      continue;
    }

    // Iterate through the lines in reverse order (newest to oldest).
    for line in text.lines().rev() {
      if cancel.is_canceled() {
        return LayoutJob::default();
      }

      // Split the date and text.
      let (datetime, mut text) = get_log_datetime_and_text(line);

      // Search the text portion.
      let mut find = search.find_in(text);
      if find.is_none() {
        continue;
      };

      let mut pos = layout.text.len();

      if !datetime.is_empty() {
        // Highlight the date/time.
        layout.text.push_str(datetime);
        layout.sections.push(LayoutSection {
          leading_space: 0.0,
          byte_range: pos..pos + datetime.len(),
          format: format_datetime.clone(),
        });
        pos += datetime.len();
      }

      layout.text.push_str(text);
      layout.text.push('\n');

      while let Some(range) = find {
        let start = pos + range.start;
        let end = pos + range.end;

        if start > pos {
          // Text before the match.
          layout.sections.push(LayoutSection {
            leading_space: 0.0,
            byte_range: pos..start,
            format: format_normal.clone(),
          });
        }

        // Highlight the match
        layout.sections.push(LayoutSection {
          leading_space: 0.0,
          byte_range: start..end,
          format: format_match.clone(),
        });

        pos += range.end;
        text = &text[range.end..];

        // Search for another match.
        find = search.find_in(text);
      }

      // The rest.
      layout.sections.push(LayoutSection {
        leading_space: 0.0,
        byte_range: pos..pos + text.len() + 1,
        format: format_normal.clone(),
      });

      if layout.text.len() >= LOG_SEARCH_LIMIT {
        return layout;
      }
    }
  }

  layout
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
  let filenames: Vec<Box<str>> = {
    let begin = span.begin.date();
    let end = span.end.date();

    // Filter the filenames to the date range.
    get_log_filenames(&log_path, Some(&avatar), None)
      .into_iter()
      .filter(|filename| {
        let path = Path::new(filename.as_ref());
        if let Some(date) = get_log_file_date(path) {
          return date >= begin && date <= end;
        }
        false
      })
      .collect()
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
  let begin_timestamp = span.begin.and_utc().timestamp();
  let end_timestamp = span.end.and_utc().timestamp();
  let range = if end_timestamp >= begin_timestamp {
    begin_timestamp..=end_timestamp
  } else {
    end_timestamp..=begin_timestamp
  };

  // Actual damage start and end timestamps.
  let mut dmg_start_timestamp = None;
  let mut dmg_end_timestamp = None;

  fn parse_digits(text: &str) -> Option<u64> {
    // Digits are at the end.
    let digits = text.split_whitespace().next_back()?;
    digits.parse::<u64>().ok()
  }

  for filename in filenames {
    if cancel.is_canceled() {
      return DPSTally::new(span);
    }

    // Read the log file.
    let path = log_path.join(filename.as_ref());
    let file_date = get_log_file_date(&path).unwrap();
    let Ok(text) = fs::read_to_string(path) else {
      continue;
    };

    // Search for attack lines.
    for line in text.lines() {
      let (datetime, text) = get_log_datetime_and_text(line);
      if datetime.is_empty() {
        continue;
      }

      let Some(timestamp) = log_datetime_to_timestamp(datetime, file_date) else {
        continue;
      };

      if !range.contains(&timestamp) {
        continue;
      }

      if cancel.is_canceled() {
        return DPSTally::new(span);
      }

      if let Some(found) = avatar_search.find(text) {
        let Some(value) = parse_digits(&text[found.range()]) else {
          continue;
        };
        dps_tally.avatar += value;
      } else if let Some(found) = pet_search.find(text) {
        let Some(value) = parse_digits(&text[found.range()]) else {
          continue;
        };
        dps_tally.pet += value;
      } else {
        continue;
      }

      if dmg_start_timestamp.is_none() {
        dmg_start_timestamp = Some(timestamp);
      }

      dmg_end_timestamp = Some(timestamp);
    }
  }

  if let Some(start_timestamp) = dmg_start_timestamp {
    if let Some(begin) = DateTime::from_timestamp(start_timestamp, 0) {
      // Update the begin data/time.
      dps_tally.span.begin = begin.naive_utc();
    }

    if let Some(end_timestamp) = dmg_end_timestamp {
      if let Some(end) = DateTime::from_timestamp(end_timestamp, 0) {
        // Update the end data/time.
        dps_tally.span.end = end.naive_utc();
      }
      dps_tally.secs = 0.max(end_timestamp - start_timestamp) as u64;
    }
  }

  dps_tally.secs += 1;
  dps_tally
}

/// Get separate date/time and text portions of a log entry.
/// > **NOTE:** the date/time will still have the surrounding square brackets.
pub fn get_log_datetime_and_text(line: &str) -> (&str, &str) {
  let Some(datetime) = get_log_datetime(line) else {
    return (Default::default(), line);
  };

  let text = &line[datetime.len()..];

  // Check if a chat timestamp was output.
  let trimmed = text.trim_start();
  if let Some(time) = get_log_datetime(trimmed) {
    return (datetime, &trimmed[time.len()..]);
  }

  (datetime, text)
}

fn get_sorted_log_filenames(log_path: &Path, avatar: Option<&str>) -> Vec<Box<str>> {
  let mut filenames = get_log_filenames(log_path, avatar, None);

  // Sort files from newest to oldest.
  filenames.sort_unstable_by(|a, b| b.cmp(a));
  filenames
}

fn get_log_filenames(log_path: &Path, avatar: Option<&str>, timestamp: Option<i64>) -> Vec<Box<str>> {
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
        filenames.push(filename.into());
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
  let time = NaiveTime::from_hms_opt(hour, minute, second)?;
  Some(NaiveDateTime::new(date, time).and_utc().timestamp())
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
