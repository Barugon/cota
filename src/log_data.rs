use crate::util::{self, Search};
use chrono::{NaiveDate, NaiveDateTime, NaiveTime};
use futures::{channel::mpsc, executor::ThreadPool, StreamExt};
use regex::Regex;
use std::{
  cmp::Reverse,
  collections::HashSet,
  fs,
  path::{Path, PathBuf},
  str::SplitWhitespace,
  sync::{atomic::AtomicBool, Arc},
};

/// Get the date portion of a log entry.
pub fn get_log_date(line: &str) -> Option<&str> {
  if !line.starts_with('[') {
    return None;
  }

  let pos = line.find(']')?;
  Some(&line[0..=pos])
}

/// Get the text portion (sans date and time) of a log entry.
pub fn get_log_text(line: &str) -> &str {
  if let Some(date) = get_log_date(line) {
    let text = &line[date.len()..];

    // Check if a chat timestamp was output.
    if let Some(time) = get_log_date(text.trim_start()) {
      let pos = util::offset(text, time).unwrap() + time.len();
      return &text[pos..];
    }
    return text;
  }

  line
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

const FILENAME_START: &str = "SotAChatLog_";
const STATS_KEY: &str = " AdventurerLevel: ";
const LOG_SEARCH_LIMIT: usize = 256 * 1024;

/// Get a vector of avatar names from the log file names.
pub async fn get_avatars(log_path: PathBuf, cancel: Arc<AtomicBool>) -> Vec<String> {
  let filenames = get_log_filenames(&log_path, None, None);
  let mut name_set = HashSet::new();

  for filename in &filenames {
    canceled!(cancel, Vec::new());
    let filename = &filename[FILENAME_START.len()..];
    if let Some(pos) = filename.rfind('_') {
      name_set.insert(&filename[..pos]);
    }
  }

  let mut avatars = Vec::with_capacity(name_set.len());
  for name in name_set {
    canceled!(cancel, Vec::new());
    avatars.push(String::from(name));
  }

  canceled!(cancel, Vec::new());

  // Sort the avatars.
  avatars.sort_unstable();
  avatars
}

/// Get a vector of timestamps where `/stats` was used for the specified avatar.
pub async fn get_stats_timestamps(
  log_path: PathBuf,
  avatar: String,
  cancel: Arc<AtomicBool>,
  thread_pool: Option<Arc<ThreadPool>>,
) -> Vec<i64> {
  // Collect the futures, one for each matching log file.
  let futures = {
    let filenames = get_log_filenames(&log_path, Some(&avatar), None);
    let mut futures = Vec::with_capacity(filenames.len());

    for filename in filenames {
      canceled!(cancel, Vec::new());
      let path = log_path.join(filename.as_str());
      let cancel = cancel.clone();
      futures.push(async move {
        let date = some!(get_log_file_date(&path), Vec::new());
        let text = ok!(fs::read_to_string(&path), Vec::new());
        let mut timestamps = Vec::new();

        for line in text.lines() {
          canceled!(cancel, Vec::new());
          if let Some(ts) = get_stats_timestamp(line, date) {
            timestamps.push(ts);
          }
        }

        timestamps
      });
    }

    futures
  };

  let (results, size) = if let Some(thread_pool) = thread_pool {
    // Process each future on a pooled thread.
    let count = futures.len();
    let mut rx = {
      let (tx, rx) = mpsc::unbounded();
      for future in futures {
        let tx = tx.clone();
        thread_pool.spawn_ok(async move {
          let result = future.await;
          tx.unbounded_send(result).unwrap();
        });
      }
      rx
    };

    // Collect the results from the MPSC receiver.
    let mut size = 0;
    let mut results = Vec::with_capacity(count);
    while let Some(result) = rx.next().await {
      canceled!(cancel, Vec::new());
      size += result.len();
      results.push(result);
    }

    (results, size)
  } else {
    // Collect the results directly.
    let mut size = 0;
    let mut results = Vec::with_capacity(futures.len());
    for future in futures {
      canceled!(cancel, Vec::new());
      let result = future.await;
      size += result.len();
      results.push(result);
    }

    (results, size)
  };

  canceled!(cancel, Vec::new());

  // Flatten the results.
  let mut timestamps = Vec::with_capacity(size);
  for mut result in results {
    timestamps.append(&mut result);
  }

  canceled!(cancel, Vec::new());

  // Sort the timestamps so that the most recent is first.
  timestamps.sort_unstable_by_key(|&key| Reverse(key));
  timestamps
}

/// Get the stats for the specified avatar and timestamp.
pub async fn get_stats(
  log_path: PathBuf,
  avatar: String,
  ts: i64,
  cancel: Arc<AtomicBool>,
) -> StatsData {
  if !avatar.is_empty() {
    let filenames = get_log_filenames(&log_path, Some(&avatar), Some(ts));

    // There will actually only be one file with the specific avatar name and date.
    for filename in filenames {
      let path = log_path.join(filename.as_str());
      if let Some(date) = get_log_file_date(&path) {
        if let Ok(text) = fs::read_to_string(path) {
          // Find the line with the specific date/time.
          for line in text.lines() {
            canceled!(cancel, StatsData::default());
            if let Some(mut stats) = get_stats_text(line, ts, date) {
              // Include subsequent lines that do not start with a square bracket.
              let pos = util::offset(&text, stats).unwrap();
              let sub = &text[pos + stats.len()..];
              for line in sub.lines() {
                if line.starts_with('[') {
                  break;
                }
                stats = &text[pos..util::offset(&text, line).unwrap()];
              }

              return StatsData::new(stats.into());
            }
          }
        }
      }
    }
  }

  StatsData::default()
}

/// Find log entries matching the provided search term.
pub async fn find_log_entries(
  log_path: PathBuf,
  avatar: String,
  search: Search,
  cancel: Arc<AtomicBool>,
) -> String {
  // Work on files from newest to oldest.
  let filenames = {
    let mut filenames = get_log_filenames(&log_path, Some(&avatar), None);
    filenames.sort_unstable_by(|a, b| b.cmp(a));
    filenames
  };

  let mut results = Vec::new();
  let mut total_size: usize = 0;
  for filename in filenames {
    canceled!(cancel, String::new());
    let path = log_path.join(filename);
    if let Ok(text) = fs::read_to_string(&path) {
      if text.is_empty() || !verify_log_text(&text) {
        continue;
      }

      let mut lines = Vec::new();
      let mut alloc_size: usize = 0;

      // Iterate through the lines in reverse order (newest to oldest).
      for line in text.lines().rev() {
        canceled!(cancel, String::new());
        if search.find_in(line).is_none() {
          continue;
        }

        // Filter out superfluous chat timestamp.
        let (date, text) = if let Some(date) = get_log_date(line) {
          (date, get_log_text(line))
        } else {
          (Default::default(), line)
        };

        let size = date.len() + text.len();
        if size > 0 {
          // Account for a newline.
          let size = size + 1;
          alloc_size += size;
          total_size += size;
          lines.push((date, text));
        }

        if total_size >= LOG_SEARCH_LIMIT {
          break;
        }
      }

      // Push all the matching lines to a new string.
      let mut concatenated = String::with_capacity(alloc_size);
      for (date, text) in lines {
        canceled!(cancel, String::new());
        concatenated.push_str(date);
        concatenated.push_str(text);
        concatenated.push('\n');
      }
      results.push(concatenated);
    }

    if total_size >= LOG_SEARCH_LIMIT {
      break;
    }
  }

  // Concatenate the results.
  let mut text = String::with_capacity(total_size);
  for result in results {
    canceled!(cancel, String::new());
    text.push_str(&result);
  }

  text
}

fn get_log_filenames(log_path: &Path, avatar: Option<&str>, ts: Option<i64>) -> Vec<String> {
  let mut filenames = Vec::new();
  let entries = ok!(log_path.read_dir(), filenames);

  // The name text is either a specific avatar or, if not specified, a regex wildcard.
  let name = if let Some(avatar) = avatar {
    avatar
  } else {
    ".+"
  };

  // The date text is either a specific date or, if not specified, regex to match the date.
  let date = if let Some(ts) = ts {
    format!("_{}", timestamp_to_file_date(ts))
  } else {
    String::from(r"_\d{4}-\d{2}-\d{2}")
  };

  let regex = ok!(
    Regex::new(&format!("^{}{}{}.txt$", FILENAME_START, name, date)),
    filenames
  );

  for entry in entries.flatten() {
    if let Ok(filename) = entry.file_name().into_string() {
      if regex.is_match(&filename) {
        filenames.push(filename);
      }
    }
  }

  filenames
}

/// Make sure the text contains at least one date/time.
fn verify_log_text(text: &str) -> bool {
  for line in text.lines() {
    if get_log_date(line).is_some() {
      return true;
    }
  }
  false
}

/// Convert a SotA log date & time into a timestamp. Since the dates are localized, we don't know
/// if day or month come first, so we use the date from the filename, which is always YYYY-MM-DD.
fn log_date_to_timestamp(text: &str, date: NaiveDate) -> Option<i64> {
  let mut iter = text.split_whitespace();
  let _date = iter.next()?;
  let time = iter.next()?;
  let ap = iter.next();

  // Parse the hour and adjust for PM.
  let mut iter = time.split(':');
  let hour = {
    let mut hour = iter.next()?.parse().ok()?;
    if let Some(ap) = ap {
      if let Some(ch) = ap.chars().next() {
        if ch == 'P' || ch == 'p' {
          hour += 12;
          if hour == 24 {
            hour = 0;
          }
        }
      }
    }
    hour
  };

  let minute = iter.next()?.parse().ok()?;
  let second = iter.next()?.parse().ok()?;

  Some(NaiveDateTime::new(date, NaiveTime::from_hms(hour, minute, second)).timestamp())
}

/// Convert a timestamp into a log filename date string.
fn timestamp_to_file_date(ts: i64) -> String {
  NaiveDateTime::from_timestamp(ts, 0)
    .format("%Y-%m-%d")
    .to_string()
}

/// Get a NaiveDate from a log filename.
fn get_log_file_date(path: &Path) -> Option<NaiveDate> {
  let filename = path.file_stem()?;
  let filename = filename.to_str()?;
  let pos = filename.rfind('_')?;
  let text = &filename[pos + 1..];
  NaiveDate::parse_from_str(text, "%Y-%m-%d").ok()
}

/// Get the log entry date as a timestamp if it's a `/stats` entry.
fn get_stats_timestamp(line: &str, file_date: NaiveDate) -> Option<i64> {
  let date = get_log_date(line)?;
  if line[date.len()..].contains(STATS_KEY) {
    return log_date_to_timestamp(&date[1..date.len() - 1], file_date);
  }

  None
}

/// Get the log entry text if it's `/stats` and the date/time matches.
fn get_stats_text(line: &str, ts: i64, file_date: NaiveDate) -> Option<&str> {
  let lts = get_stats_timestamp(line, file_date)?;
  if lts == ts {
    return Some(get_log_text(line));
  }

  None
}
