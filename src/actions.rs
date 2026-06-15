// Copyright 2026 Merck KGaA, Darmstadt, Germany and/or its affiliates.
// All rights reserved

//! # Actions
//!
//! This module provides functionality for executing OPC UA operations defined
//! in a CSV script file. Each row in the CSV describes a single action to
//! perform against an OPC UA server — such as reading a node value, writing a
//! value, waiting for user input, or polling until a node reaches a target
//! state.
//!
//! ## CSV Format
//!
//! The CSV file must contain the following columns (order matters, headers required):
//!
//! | Column  | Type             | Description                                                            |
//! |---------|------------------|------------------------------------------------------------------------|
//! | action  | `String`         | The operation to perform (see supported actions below)                 |
//! | tag     | `String`         | The OPC UA node identifier string (namespace index 2)                  |
//! | value   | `Option<String>` | Optional value used by `write`, `user_write`, and `wait_until`         |
//! | sleep   | `u64`            | Milliseconds to sleep after the row is processed                       |
//!
//! ## Supported Actions
//!
//! | Action        | Description                                                                                 |
//! |---------------|---------------------------------------------------------------------------------------------|
//! | `read`        | Reads the current value of the OPC UA node and prints it to stdout.                         |
//! | `write`       | Writes the value in the `value` column to the OPC UA node.                                  |
//! | `user_write`  | Prompts the user for input (or uses `value` if provided) and writes it to the node.         |
//! | `comment`     | Prints the `tag` column as a human-readable comment — no OPC UA interaction.                |
//! | `wait`        | Pauses execution and waits for the user to press Enter before continuing.                   |
//! | `wait_until`  | Polls the OPC UA node every `sleep` ms until its value equals `value`.                      |
//! | `#...`        | Any action starting with `#` is a silent inline script comment and is skipped.              |
//!
//! ## Value Parsing
//!
//! Values in the `value` column are automatically parsed into OPC UA `Variant`
//! types by [`parse_variant`]:
//!
//! - `"true"` / `"false"` (case-insensitive) → `Variant::Boolean`
//! - Integer strings (e.g. `"42"`) → `Variant::Int32`
//! - Floating-point strings (e.g. `"3.14"`) → `Variant::Double`
//! - Anything else → `Variant::String`
//!
//! ## Example CSV
//!
//! ```text
//! action,tag,value,sleep
//! comment,Starting test sequence,,0
//! write,Reactor/SetPoint,75.0,500
//! wait_until,Reactor/Status,Running,1000
//! read,Reactor/Temperature,,0
//! wait,Press Enter to continue,,0
//! ```
//!
//! ## Usage
//!
//! ```rust,ignore
//! use my_crate::csv_runner::run_csv;
//!
//! run_csv(&mut opc_client, &mut stdin_reader, "./script.csv");
//! ```

use color_print::{cprintln, cformat};
use opcua_client::prelude::{NodeId, UAString, Variant};
use serde::Deserialize;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use crate::globals::Globals;
use crate::logger::Logger;
use crate::opc_ua_client::OpcUaClient;
use crate::reader::InputReader;

/// Represents a single row in the CSV script file.
///
/// Each field maps directly to a CSV column. The struct is deserialized
/// automatically by [`run_csv`] using the `csv` + `serde` crates.
///
/// # Fields
///
/// - `action` — The operation to execute (e.g. `"read"`, `"write"`).
/// - `tag`    — The OPC UA node identifier string. A [`NodeId`] is constructed
///              from this using namespace index `2`.
/// - `value`  — An optional string value. Required by `write`, `user_write`,
///              and `wait_until`; ignored by others.
/// - `sleep`  — Milliseconds to sleep **after** the row has been processed.
///              Also used as the polling interval in `wait_until`.
#[derive(Debug, Deserialize, PartialEq)]
pub struct CsvRow {
    /// The action keyword that determines which OPC UA operation is performed.
    pub action: String,
    /// The OPC UA node tag / identifier string (namespace index 2).
    pub tag: String,
    /// An optional value string, interpreted by [`parse_variant`].
    pub value: Option<String>,
    /// Milliseconds to sleep after processing this row (and polling interval
    /// for `wait_until`).
    pub sleep: u64,
}

fn split_first(s: &str) -> Option<(char, &str)> {
    let mut chars = s.chars();
    let first = chars.next()?;
    Some((first, chars.as_str()))
}

/// Parses a string slice into an OPC UA [`Variant`].
///
/// The conversion follows this priority order:
///
/// 1. `$*` -> [`Variant::String`]
/// 2. `"true"` or `"false"` (case-insensitive, whitespace trimmed)
///    → [`Variant::Boolean`]
/// 3. A valid `i32` integer literal → [`Variant::Int32`]
/// 4. A valid `f64` floating-point literal → [`Variant::Double`]
/// 5. Anything else → [`Variant::String`] (whitespace trimmed)
///
/// # Examples
///
/// ```rust,ignore
/// assert_eq!(parse_variant("true"),  Variant::Boolean(true));
/// assert_eq!(parse_variant("42"),    Variant::Int32(42));
/// assert_eq!(parse_variant("3.14"),  Variant::Double(3.14));
/// assert_eq!(parse_variant("hello"), Variant::String(UAString::from("hello")));
/// ```
pub fn parse_variant(s: &str) -> Variant {
    if s.starts_with("$") {
        if let Some((_, remainder)) = split_first(s) {
            return Variant::String(UAString::from(remainder.trim()))
        }
    }
    let lower = s.trim().to_lowercase();

    if lower == "true" {
        return Variant::Boolean(true);
    }
    if lower == "false" {
        return Variant::Boolean(false);
    }
    if let Ok(i) = s.trim().parse::<i32>() {
        return Variant::Int32(i);
    }
    if let Ok(f) = s.trim().parse::<f64>() {
        return Variant::Double(f);
    }
    Variant::String(UAString::from(s.trim()))
}

/// Coerces a raw string into the same [`Variant`] discriminant as `template`.
///
/// Used to match the server's actual datatype rather than guessing from the
/// string content.  Falls back to [`parse_variant`] if the target type is
/// unknown or the conversion fails.
///
/// # Arguments
/// * `s`        — The raw value string from the CSV.
/// * `template` — A [`Variant`] whose discriminant defines the target type.
pub fn coerce_variant(s: &str, template: &Variant) -> Variant {
    let t = s.trim();
    match template {
        Variant::Boolean(_) => {
            match t.to_lowercase().as_str() {
                "true" | "1"  => Variant::Boolean(true),
                "false" | "0" => Variant::Boolean(false),
                _             => parse_variant(t),   // fallback
            }
        }
        Variant::SByte(_)  => t.parse::<i8>()  .map(Variant::SByte) .unwrap_or_else(|_| parse_variant(t)),
        Variant::Byte(_)   => t.parse::<u8>()  .map(Variant::Byte)  .unwrap_or_else(|_| parse_variant(t)),
        Variant::Int16(_)  => t.parse::<i16>() .map(Variant::Int16) .unwrap_or_else(|_| parse_variant(t)),
        Variant::UInt16(_) => t.parse::<u16>() .map(Variant::UInt16).unwrap_or_else(|_| parse_variant(t)),
        Variant::Int32(_)  => t.parse::<i32>() .map(Variant::Int32) .unwrap_or_else(|_| parse_variant(t)),
        Variant::UInt32(_) => t.parse::<u32>() .map(Variant::UInt32).unwrap_or_else(|_| parse_variant(t)),
        Variant::Int64(_)  => t.parse::<i64>() .map(Variant::Int64) .unwrap_or_else(|_| parse_variant(t)),
        Variant::UInt64(_) => t.parse::<u64>() .map(Variant::UInt64).unwrap_or_else(|_| parse_variant(t)),
        Variant::Float(_)  => t.parse::<f32>() .map(Variant::Float) .unwrap_or_else(|_| parse_variant(t)),
        Variant::Double(_) => t.parse::<f64>() .map(Variant::Double).unwrap_or_else(|_| parse_variant(t)),
        Variant::String(_) => Variant::String(UAString::from(t)),
        _                  => parse_variant(t),   // DateTime, NodeId, etc. — best-effort
    }
}

/// Executes a single [`CsvRow`] against the provided OPC UA client.
///
/// This is the core dispatch function of the module. It inspects
/// `row.action` and routes to the appropriate OPC UA operation.
///
/// # Arguments
///
/// * `row`    — The parsed CSV row describing the action to perform.
/// * `line`   — The 1-based CSV line number (used in error messages).
/// * `client` — A mutable reference to any type implementing [`OpcUaClient`].
/// * `reader` — A mutable reference to any type implementing [`InputReader`],
///              used for `wait` and `user_write` prompts.
///
/// # Behaviour
///
/// After the action is executed, the function sleeps for `row.sleep`
/// milliseconds (if > 0).
///
/// Unknown action strings are logged as warnings and skipped rather than
/// panicking.
pub fn process_row(
    row:    &CsvRow,
    line:   usize,
    client: &mut impl OpcUaClient,
    reader: &mut impl InputReader,
    logger: &Arc<Mutex<Logger>>,
) {
    let _out = |msg: &str| {
        println!("{}", msg);
        if let Ok(log) = logger.lock() {
            log.log_output(msg);
        }
    };

    let node_id = NodeId::new(2, row.tag.clone());

    match row.action.trim().to_lowercase().as_str() {
        s if s.starts_with('#') => { /* inline script comment — silent */ }

        "read" => match client.read(&node_id) {
            Some(v) => {
                let msg = Globals::csv_read_ok(&row.tag, &format!("{:?}", v));
                cprintln!("<green>{}</>", msg);
                if let Ok(log) = logger.lock() { log.log_output(&msg); }
            }
            None => {
                let msg = Globals::csv_read_no_value(&row.tag);
                cprintln!("<yellow>{}</>", msg);
                if let Ok(log) = logger.lock() { log.log_output(&msg); }
            }
        },

        "write" => match &row.value {
            Some(v_str) => {
                let variant = match client.read(&node_id) {
                    Some(current) => coerce_variant(v_str, &current),
                    None          => parse_variant(v_str),   // node unreadable — best-effort
                };
                let msg = Globals::csv_write(&row.tag, &format!("{:?}", variant));
                cprintln!("<bright-green>{}</>", msg);
                if let Ok(log) = logger.lock() { log.log_output(&msg); }
                client.write(&node_id, variant);
            }
            None => {
                let msg = Globals::csv_write_missing_value(line, &row.tag);
                cprintln!("<bright-yellow>{}</>", msg);
                if let Ok(log) = logger.lock() { log.log_output(&msg); }
            }
        },

        "user_write" => {
            let raw = match &row.value {
                Some(v_str) => {
                    let msg = Globals::csv_user_write(&row.tag, v_str);
                    cprintln!("<bright-green>{}</>", msg);
                    if let Ok(log) = logger.lock() { log.log_output(&msg); }
                    v_str.clone()
                }
                None => reader.read_line(
                    cformat!("<bright-green>{}</>", Globals::csv_user_write_prompt(&row.tag))
                ),
            };

            let variant = match client.read(&node_id) {
                Some(current) => coerce_variant(&raw, &current),
                None          => parse_variant(&raw),
            };
            client.write(&node_id, variant);
        }

        "comment" => {
            let msg = Globals::csv_comment(&row.tag);
            cprintln!("<white>{}</>", msg);
            if let Ok(log) = logger.lock() { log.log_output(&msg); }
        }

        "wait" => {
            // The prompt itself is logged as INPUT by StdinReader.
            reader.read_line(cformat!("<white>{}</>", Globals::csv_wait(&row.tag)));
        }

        "wait_until" => {
            if let Some(v_str) = &row.value {
                let target = match client.read(&node_id) {
                    Some(current) => coerce_variant(v_str, &current),
                    None          => parse_variant(v_str),
                };
                let mut waiting_message_shown = false;

                loop {
                    match client.read(&node_id) {
                        Some(current) => {
                            if current == target {
                                let msg = Globals::csv_wait_until_completed(&row.tag, &current);
                                cprintln!("<green>{}</>", msg);
                                if let Ok(log) = logger.lock() { log.log_output(&msg); }
                                break;
                            } else if !waiting_message_shown {
                                let msg = Globals::csv_wait_until(&row.tag, &current, &target);
                                cprintln!("<white>{}</>", msg);
                                if let Ok(log) = logger.lock() { log.log_output(&msg); }
                                waiting_message_shown = true;
                            }
                        }
                        None => {
                            if !waiting_message_shown {
                                let msg = Globals::csv_write_missing_value(line, &row.tag);
                                cprintln!("<bright-yellow>{}</>", msg);
                                if let Ok(log) = logger.lock() { log.log_output(&msg); }
                                waiting_message_shown = true;
                            }
                        }
                    }
                    std::thread::sleep(Duration::from_millis(row.sleep.max(1)));
                }
            } else {
                let msg = Globals::csv_write_missing_value(line, &row.tag);
                cprintln!("<bright-yellow>{}</>", msg);
                if let Ok(log) = logger.lock() { log.log_output(&msg); }
            }
        }

        other => {
            let msg = Globals::csv_unknown_action(line, other);
            cprintln!("<yellow>{}</>", msg);
            if let Ok(log) = logger.lock() { log.log_output(&msg); }
        }
    }

    if row.sleep > 0 {
        thread::sleep(Duration::from_millis(row.sleep));
    }
}

/// Reads and executes a CSV script file against the provided OPC UA client.
///
/// Opens the file at `csv_path`, deserializes each row into a [`CsvRow`],
/// and dispatches it to [`process_row`]. Rows that fail to deserialize are
/// printed as errors to `stderr` and skipped — they do **not** abort the run.
///
/// # Arguments
///
/// * `client`   — A mutable reference to any type implementing [`OpcUaClient`].
/// * `reader`   — A mutable reference to any type implementing [`InputReader`].
/// * `csv_path` — Path to the CSV script file on the filesystem.
///
/// # Panics
///
/// Panics if the CSV file cannot be opened (e.g. file not found, permission
/// denied). The panic message includes the path and the underlying I/O error.
///
/// # Examples
///
/// ```rust,ignore
/// run_csv(&mut opc_client, &mut stdin_reader, "./automation_script.csv");
/// ```
pub fn run_csv(
    client:   &mut impl OpcUaClient,
    reader:   &mut impl InputReader,
    csv_path: &str,
    logger:   &Arc<Mutex<Logger>>,
) {
    let mut rdr = csv::ReaderBuilder::new()
        .trim(csv::Trim::All)
        .from_path(csv_path)
        .unwrap_or_else(|e| panic!("{}", Globals::csv_open_failed(csv_path, e)));

    for (line, result) in rdr.deserialize::<CsvRow>().enumerate() {
        match result {
            Ok(row)  => process_row(&row, line + 2, client, reader, logger),
            Err(e)   => eprintln!("{}", Globals::csv_invalid_row(line + 2, e)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use opcua_client::prelude::{NodeId, Variant, UAString};
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};
    use crate::logger::Logger;
    use crate::opc_ua_client::OpcUaClient;
    use crate::reader::InputReader;

    fn no_logger() -> Arc<Mutex<Logger>> {
        Arc::new(Mutex::new(Logger::new(None)))
    }

    // ── Fakes ────────────────────────────────────────────────────────────────

    #[derive(Default)]
    struct FakeClient {
        pub store:  HashMap<String, Variant>,
        pub writes: Vec<(NodeId, Variant)>,
    }

    impl OpcUaClient for FakeClient {
        fn read(&self, node_id: &NodeId) -> Option<Variant> {
            self.store.get(&node_id.to_string()).cloned()
        }
        fn write(&mut self, node_id: &NodeId, value: Variant) {
            self.writes.push((node_id.clone(), value));
        }
    }

    struct ScriptedReader { lines: Vec<String> }

    impl ScriptedReader {
        fn new(lines: &[&str]) -> Self {
            Self { lines: lines.iter().rev().map(|s| s.to_string()).collect() }
        }
    }

    impl InputReader for ScriptedReader {
        fn read_line(&mut self, _prompt: String) -> String {
            self.lines.pop().unwrap_or_default()
        }
    }

    // ── parse_variant tests ───────────────────────────────────────────────────

    #[test]
    fn write_row_calls_client() {
        let mut client = FakeClient::default();
        // Seed so that the pre-read returns the server type (Int32)
        client.store.insert(
            NodeId::new(2, "MyTag").to_string(),
            Variant::Int32(0),
        );
        let mut reader = ScriptedReader::new(&[]);
        let row = CsvRow { action: "write".into(), tag: "MyTag".into(),
                           value: Some("99".into()), sleep: 0 };
        process_row(&row, 2, &mut client, &mut reader, &no_logger());
        assert_eq!(client.writes.len(), 1);
        assert_eq!(client.writes[0].1, Variant::Int32(99));
    }

    #[test]
    fn write_row_unseedable_node_falls_back_to_parse() {
        // Node not in store → read returns None → parse_variant fallback
        let mut client = FakeClient::default();
        let mut reader = ScriptedReader::new(&[]);
        let row = CsvRow { action: "write".into(), tag: "MyTag".into(),
                           value: Some("99".into()), sleep: 0 };
        process_row(&row, 2, &mut client, &mut reader, &no_logger());
        assert_eq!(client.writes[0].1, Variant::Int32(99));  // parse_variant gives Int32
    }

    #[test]
    fn write_coerces_to_server_float() {
        let mut client = FakeClient::default();
        client.store.insert(
            NodeId::new(2, "MyTag").to_string(),
            Variant::Float(0.0),   // server uses Float, not Double
        );
        let mut reader = ScriptedReader::new(&[]);
        let row = CsvRow { action: "write".into(), tag: "MyTag".into(),
                           value: Some("3.14".into()), sleep: 0 };
        process_row(&row, 2, &mut client, &mut reader, &no_logger());
        assert_eq!(client.writes[0].1, Variant::Float(3.14_f32));
    }


    // ── process_row tests ────────────────────────────────────────────────────

    #[test]
    fn user_write_reads_from_reader() {
        let mut client = FakeClient::default();
        let mut reader = ScriptedReader::new(&["123"]);
        let row = CsvRow { action: "user_write".into(), tag: "MyTag".into(),
                           value: None, sleep: 0 };
        process_row(&row, 2, &mut client, &mut reader, &no_logger());
        assert_eq!(client.writes[0].1, Variant::Int32(123));
    }

    #[test]
    fn read_row_with_no_value_does_not_write() {
        let mut client = FakeClient::default();
        let mut reader = ScriptedReader::new(&[]);
        let row = CsvRow { action: "read".into(), tag: "Missing".into(),
                           value: None, sleep: 0 };
        process_row(&row, 2, &mut client, &mut reader, &no_logger());
        assert!(client.writes.is_empty());
    }
}
