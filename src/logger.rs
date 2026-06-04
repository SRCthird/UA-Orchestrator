// Copyright 2026 Merck KGaA, Darmhardt, Germany and/or its affiliates.
// All rights reserved

//! # Logger
//!
//! Optional append-only log file shared across the reader and action runner.
//!
//! ## Format
//!
//! Every entry is a single UTF-8 line:
//!
//! ```text
//! 2026-06-04T14:32:01Z | INPUT  | Enter CSV path:  | /data/run42.csv
//! 2026-06-04T14:32:02Z | OUTPUT | [Read] Reactor/Temperature → 73.2
//! ```
//!
//! Fields are separated by ` | `:
//!
//! 1. UTC timestamp (`YYYY-MM-DDTHH:MM:SSZ`)
//! 2. Kind — `INPUT` or `OUTPUT`
//! 3. Payload (prompt + user text for inputs; plain message for outputs)
//!
//! ## Usage
//!
//! ```rust,ignore
//! use std::sync::{Arc, Mutex};
//! use crate::logger::Logger;
//!
//! let logger = Arc::new(Mutex::new(Logger::new(Some("/var/log/ua.log".into()))));
//!
//! logger.lock().unwrap().log_output("Hello from actions");
//! logger.lock().unwrap().log_input("Prompt text", "user answer");
//! ```

use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;

/// Identifies whether a log record came from user input or program output.
pub enum LogKind {
    Input,
    Output,
}

/// Append-only log file writer.
///
/// Constructed once and shared via `Arc<Mutex<Logger>>`. All writes are
/// fail-silent: errors are printed to `stderr` but never propagate.
pub struct Logger {
    log_path: Option<PathBuf>,
}

impl Logger {
    /// Creates a new [`Logger`].
    ///
    /// # Arguments
    /// * `log_path` — `Some(path)` to enable logging; `None` to disable it
    ///   entirely.  The file is created if absent; parent directories must
    ///   already exist.
    pub fn new(log_path: Option<PathBuf>) -> Self {
        Self { log_path }
    }

    /// Returns `true` if a log path has been configured.
    pub fn is_enabled(&self) -> bool {
        self.log_path.is_some()
    }

    /// Appends a record to the log file.
    ///
    /// # Arguments
    /// * `kind`    — [`LogKind::Input`] or [`LogKind::Output`].
    /// * `payload` — The message to record (ANSI codes should be stripped by
    ///               the caller).
    pub fn write(&self, kind: LogKind, payload: &str) {
        let Some(ref path) = self.log_path else { return };

        let kind_str = match kind {
            LogKind::Input  => "INPUT ",
            LogKind::Output => "OUTPUT",
        };

        let timestamp = epoch_to_iso8601(unix_now());

        match OpenOptions::new().create(true).append(true).open(path) {
            Ok(mut file) => {
                if let Err(e) = writeln!(file, "{} | {} | {}", timestamp, kind_str, payload) {
                    eprintln!("Warning: failed to write to log file {:?}: {e}", path);
                }
            }
            Err(e) => {
                eprintln!("Warning: could not open log file {:?}: {e}", path);
            }
        }
    }

    /// Convenience wrapper for user input records.
    ///
    /// Writes a single `INPUT` entry formatted as `"<prompt> | <input>"`.
    pub fn log_input(&self, plain_prompt: &str, input: &str) {
        self.write(LogKind::Input, &format!("{} | {}", plain_prompt.trim(), input));
    }

    /// Convenience wrapper for program output records.
    ///
    /// Strips ANSI codes from `msg` before writing so the log file stays
    /// human-readable without escape sequences.
    pub fn log_output(&self, msg: &str) {
        self.write(LogKind::Output, &strip_ansi(msg));
    }
}

// ── Timestamp helpers ────────────────────────────────────────────────────────

fn unix_now() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

/// Converts a UNIX timestamp (seconds since epoch) to an RFC 3339 UTC string.
///
/// Handles all dates from 1970 through ~2100 correctly, including leap years.
/// Avoids a `chrono` dependency.
pub fn epoch_to_iso8601(epoch_secs: u64) -> String {
    const SECS_PER_MIN: u64 = 60;
    const SECS_PER_HOUR: u64 = 3600;
    const SECS_PER_DAY: u64 = 86400;

    let time_of_day = epoch_secs % SECS_PER_DAY;
    let hour   = time_of_day / SECS_PER_HOUR;
    let minute = (time_of_day % SECS_PER_HOUR) / SECS_PER_MIN;
    let second = time_of_day % SECS_PER_MIN;

    let mut days = epoch_secs / SECS_PER_DAY;
    let mut year = 1970u64;
    loop {
        let days_in_year = if is_leap(year) { 366 } else { 365 };
        if days < days_in_year { break; }
        days -= days_in_year;
        year += 1;
    }

    let month_days: [u64; 12] = [
        31, if is_leap(year) { 29 } else { 28 },
        31, 30, 31, 30, 31, 31, 30, 31, 30, 31,
    ];
    let mut month = 1u64;
    for &md in &month_days {
        if days < md { break; }
        days -= md;
        month += 1;
    }

    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        year, month, days + 1, hour, minute, second
    )
}

fn is_leap(year: u64) -> bool {
    (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
}

/// Strips ANSI SGR escape sequences from a string.
fn strip_ansi(s: &str) -> String {
    // Same regex used in reader.rs — kept local to avoid a cross-module dep.
    let re = regex::Regex::new(r"\x1b\[[0-9;]*m").unwrap();
    re.replace_all(s, "").to_string()
}
