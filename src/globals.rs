// Copyright 2026 Merck KGaA, Darmstadt, Germany and/or its affiliates.
// All rights reserved

#![allow(dead_code)]

//! # Globals
//!
//! Central message-string registry for UAOrchestrator.
//!
//! [`Globals`] is a **zero-field unit struct** whose sole purpose is to act as
//! a namespaced collection of pure functions that produce every human-readable
//! string emitted by the application. Centralising all strings here means:
//!
//! - Changing wording, prefixes, or formatting only requires editing one file.
//! - Tests can assert on exact strings without duplicating literals.
//! - Log-level changes or localisation have a single point of control.
//!
//! ## Method Groups
//!
//! | Group               | Prefix                     | Description                                      |
//! |---------------------|----------------------------|--------------------------------------------------|
//! | Configuration       | `config_`                  | TOML file loading and parsing errors             |
//! | Security            | `unsupported_`             | Unknown security policy / mode strings           |
//! | Application identity| `app_`                     | Name, URI, and startup banner lines              |
//! | Certificates        | `user_cert` / `user_key`   | Per-user PKI file paths                          |
//! | OPC UA I/O          | `read_` / `write_`         | Low-level node read/write diagnostics            |
//! | CSV runner          | `csv_`                     | All messages produced by the CSV action engine   |
//!
//! ## Example
//!
//! ```rust,ignore
//! use crate::globals::Globals;
//!
//! let path   = Globals::config_file();           // "Config.toml"
//! let banner = Globals::app_user_msg("alice");   // "Executing User   : alice"
//! ```

use opcua_client::prelude::{StatusCode,Variant};

/// Zero-field unit struct that acts as a namespace for every human-readable
/// string produced by UAOrchestrator.
///
/// All methods are **pure functions** — they accept plain data, perform string
/// formatting, and return an owned [`String`] or a `&'static str` where the
/// result is a compile-time literal. No state is mutated and no I/O is
/// performed.
pub struct Globals;

impl Globals {
    /// Creates a new [`Globals`] instance.
    ///
    /// Because [`Globals`] is a unit struct with no fields this is equivalent
    /// to the expression `Globals`. It exists for consistency with
    /// dependency-injection patterns used elsewhere in the codebase.
    pub fn new() -> Self {
        Globals
    }

    // -------------------------------------------------------------------------
    // Configuration
    // -------------------------------------------------------------------------

    /// Returns the default configuration file name.
    ///
    /// Used as the fallback path when no explicit config path is supplied at
    /// start-up.
    ///
    /// # Returns
    /// `"Config.toml"`
    pub fn config_file() -> &'static str {
        "Config.toml"
    }

    /// Formats an error message for a TOML file that could not be read.
    ///
    /// # Arguments
    /// * `path` — The filesystem path that was attempted.
    /// * `e`    — The underlying [`std::io::Error`].
    ///
    /// # Returns
    /// `"Failed to read {path}: {e}"`
    pub fn config_failed_read(path: &str, e: std::io::Error) -> String {
        format!("Failed to read {}: {}", path, e)
    }

    /// Formats an error message for a TOML file that could not be parsed.
    ///
    /// # Arguments
    /// * `path` — The filesystem path that was attempted.
    /// * `e`    — The underlying [`toml::de::Error`].
    ///
    /// # Returns
    /// `"Failed to parse {path}: {e}"`
    pub fn config_failed_parse(path: &str, e: toml::de::Error) -> String {
        format!("Failed to parse {}: {}", path, e)
    }

    // -------------------------------------------------------------------------
    // Security
    // -------------------------------------------------------------------------

    /// Formats an error message for an unrecognised OPC UA security mode.
    ///
    /// # Arguments
    /// * `other` — The unrecognised mode string from the config file.
    ///
    /// # Returns
    /// `"Unsupported security mode: {other}"`
    pub fn unsupported_security_mode(other: &str) -> String {
        format!("Unsupported security mode: {}", other)
    }

    /// Formats an error message for an unrecognised OPC UA security policy.
    ///
    /// # Arguments
    /// * `other` — The unrecognised policy string from the config file.
    ///
    /// # Returns
    /// `"Unsupported security policy: {other}"`
    pub fn unsupported_security_policy(other: &str) -> String {
        format!("Unsupported security policy: {}", other)
    }

    // -------------------------------------------------------------------------
    // Application identity
    // -------------------------------------------------------------------------

    /// Returns the interactive prompt used to request a CSV file path.
    ///
    /// # Returns
    /// `"[PATH]    Please provide path to csv <= "`
    pub fn csv_request_path() -> &'static str {
        "[PATH]    Please provide path to csv <= "
    }

    /// Builds the OPC UA application name for the current user.
    ///
    /// # Arguments
    /// * `user` — The OS username of the executing user.
    ///
    /// # Returns
    /// `"UAOrchestrator@{user}"`
    pub fn app_name(user: &str) -> String {
        format!("UAOrchestrator@{user}")
    }

    /// Builds the OPC UA application URI for the current host.
    ///
    /// # Arguments
    /// * `hostname` — The hostname of the machine running the application.
    ///
    /// # Returns
    /// `"urn:{hostname}:UAOrchestrator"`
    pub fn app_uri(hostname: &str) -> String {
        format!("urn:{hostname}:UAOrchestrator")
    }

    /// Formats the startup banner line that shows the executing user.
    ///
    /// # Returns
    /// `"Executing User   : {user}"`
    pub fn app_user_msg(user: &str) -> String {
        format!("Executing User   : {}", user)
    }

    /// Formats the startup banner line that shows the application name.
    ///
    /// # Returns
    /// `"Application Name : {app_name}"`
    pub fn app_name_msg(app_name: &str) -> String {
        format!("Application Name : {}", app_name)
    }

    /// Formats the startup banner line that shows the application URI.
    ///
    /// # Returns
    /// `"Application URI  : {app_uri}"`
    pub fn app_uri_msg(app_uri: &str) -> String {
        format!("Application URI  : {}", app_uri)
    }

    // -------------------------------------------------------------------------
    // Certificates
    // -------------------------------------------------------------------------

    /// Returns the path to the DER-encoded certificate for a given user.
    ///
    /// # Arguments
    /// * `user` — The username whose certificate is needed.
    ///
    /// # Returns
    /// `"own/cert_{user}.der"`
    pub fn user_cert(user: &str) -> String {
        format!("own/cert_{}.der", user)
    }

    /// Returns the path to the PEM-encoded private key for a given user.
    ///
    /// # Arguments
    /// * `user` — The username whose private key is needed.
    ///
    /// # Returns
    /// `"private/key_{user}.pem"`
    pub fn user_key(user: &str) -> String {
        format!("private/key_{}.pem", user)
    }

    // -------------------------------------------------------------------------
    // OPC UA I/O
    // -------------------------------------------------------------------------

    /// Returns the static error string used when no matching OPC UA endpoint
    /// can be found during connection.
    ///
    /// # Returns
    /// `"Could not find matching endpoint"`
    pub fn endpoint_error() -> &'static str {
        "Could not find matching endpoint"
    }

    /// Formats the connection banner line shown before an OPC UA session is
    /// established.
    ///
    /// # Returns
    /// `"Connecting to    : {endpoint}"`
    pub fn endpoint_connecting(endpoint: &str) -> String {
        format!("Connecting to    : {}", endpoint)
    }

    /// Formats a warning when a node read returns no data value.
    ///
    /// # Arguments
    /// * `index`  — The zero-based node index in the read request.
    /// * `status` — The OPC UA [`StatusCode`] returned by the server.
    ///
    /// # Returns
    /// `"[WARN]    Node {index}: no value, status = {status}"`
    pub fn read_no_value(index: usize, status: StatusCode) -> String {
        format!("[WARN]    Node {}: no value, status = {}", index, status)
    }

    /// Formats an error message for a failed node read operation.
    ///
    /// # Arguments
    /// * `e` — The OPC UA [`StatusCode`] describing the failure.
    ///
    /// # Returns
    /// `"[ERR]     Read failed: {e}"`
    pub fn read_error(e: StatusCode) -> String {
        format!("[ERR]     Read failed: {}", e)
    }

    /// Formats a warning when a node write returns a non-success status.
    ///
    /// # Arguments
    /// * `index`  — The zero-based node index in the write request.
    /// * `status` — The OPC UA [`StatusCode`] returned by the server.
    ///
    /// # Returns
    /// `"[WARN]    Node {index}: write status = {status}"`
    pub fn write_status(index: usize, status: StatusCode) -> String {
        format!("[WARN]    Node {}: write status = {}", index, status)
    }

    /// Formats a warning for a failed node write operation.
    ///
    /// # Arguments
    /// * `e` — The OPC UA [`StatusCode`] describing the failure.
    ///
    /// # Returns
    /// `"[WARN]    Write failed: {e}"`
    pub fn write_error(e: StatusCode) -> String {
        format!("[WARN]    Write failed: {}", e)
    }

    // -------------------------------------------------------------------------
    // CSV runner
    // -------------------------------------------------------------------------

    /// Formats the success message for a CSV `read` action.
    ///
    /// # Returns
    /// `"[READ]    {tag} => {value}"`
    pub fn csv_read_ok(tag: &str, value: &str) -> String {
        format!("[READ]    {} => {}", tag, value)
    }

    /// Formats the message shown when a CSV `read` action finds no value on
    /// the node.
    ///
    /// # Returns
    /// `"[READ]    {tag} => no value"`
    pub fn csv_read_no_value(tag: &str) -> String {
        format!("[READ]    {} => no value", tag)
    }

    /// Formats the confirmation message for a CSV `write` action.
    ///
    /// # Returns
    /// `"[WRITE]   {tag} <= {value}"`
    pub fn csv_write(tag: &str, value: &str) -> String {
        format!("[WRITE]   {} <= {}", tag, value)
    }

    /// Formats a warning for a CSV `write` or `wait_until` action whose
    /// `value` column is empty.
    ///
    /// # Arguments
    /// * `line` — The 1-based CSV line number.
    /// * `tag`  — The OPC UA node tag string.
    ///
    /// # Returns
    /// `"[WRITE]   Line {line}: missing value for write on '{tag}'"`
    pub fn csv_write_missing_value(line: usize, tag: &str) -> String {
        format!("[WRITE]   Line {}: missing value for write on '{}'", line, tag)
    }

    /// Formats the confirmation message for a CSV `user_write` action where
    /// the value was supplied in the CSV (not typed interactively).
    ///
    /// # Returns
    /// `"[UWRITE]  {tag} <= {value}"`
    pub fn csv_user_write(tag: &str, value: &str) -> String {
        format!("[UWRITE]  {} <= {}", tag, value)
    }

    /// Formats the interactive prompt displayed to the user for a CSV
    /// `user_write` action where no value is pre-specified in the CSV.
    ///
    /// # Returns
    /// `"[UWRITE]  {tag} <= "`
    pub fn csv_user_write_prompt(tag: &str) -> String {
        format!("[UWRITE]  {} <= ", tag)
    }

    /// Formats the display string for a CSV `comment` action.
    ///
    /// The `tag` column is repurposed as the comment text for this action.
    ///
    /// # Returns
    /// `"[COMMENT] {tag}"`
    pub fn csv_comment(tag: &str) -> String {
        format!("[COMMENT] {}", tag)
    }

    /// Formats a warning for an unrecognised CSV action keyword.
    ///
    /// # Arguments
    /// * `line`   — The 1-based CSV line number.
    /// * `action` — The unrecognised action string.
    ///
    /// # Returns
    /// `"[WARN]    Line {line}: unknown action '{action}', skipping"`
    pub fn csv_unknown_action(line: usize, action: &str) -> String {
        format!("[WARN]    Line {}: unknown action '{}', skipping", line, action)
    }

    /// Formats the prompt displayed to the user for a CSV `wait` action.
    ///
    /// The `tag` column is used as the wait message text.
    ///
    /// # Returns
    /// `"[WAIT]    {tag}"`
    pub fn csv_wait(tag: &str) -> String {
        format!("[WAIT]    {}", tag)
    }

    /// Formats the polling status message shown while a CSV `wait_until`
    /// action is waiting for a node to reach its target value.
    ///
    /// # Arguments
    /// * `tag`     — The OPC UA node tag string.
    /// * `current` — The current [`Variant`] value of the node.
    /// * `target`  — The target [`Variant`] value being waited for.
    ///
    /// # Returns
    /// `"[WAITFOR] {tag}:{current:?} == {target:?}"`
    pub fn csv_wait_until(tag: &str, target: &Variant, current: &Variant) -> String {
        format!("[WAITFOR] {}:{:?} == {:?}", tag, current, target)
    }

    /// Formats the completion message shown when a CSV `wait_until` action
    /// detects that the node has reached its target value.
    ///
    /// # Arguments
    /// * `tag`     — The OPC UA node tag string.
    /// * `current` — The [`Variant`] value the node settled on.
    ///
    /// # Returns
    /// `"[WAITFOR] Satisfied: {tag} == {current:?}"`
    pub fn csv_wait_until_completed(tag: &str, current: &Variant) -> String {
        format!("[WAITFOR] Satisfied: {} == {:?}", tag, current)
    }

    /// Formats an error message when the CSV file cannot be opened.
    ///
    /// # Arguments
    /// * `path` — The path to the CSV file.
    /// * `e`    — The [`csv::Error`] returned by the reader.
    ///
    /// # Returns
    /// `"[ERR]     Failed to open CSV '{path}': {e}"`
    pub fn csv_open_failed(path: &str, e: csv::Error) -> String {
        format!("[ERR]     Failed to open CSV '{}': {}", path, e)
    }

    /// Formats a warning when a CSV row fails to deserialise into a [`CsvRow`](crate::actions::CsvRow).
    ///
    /// The row is skipped and processing continues with the next line.
    ///
    /// # Arguments
    /// * `line` — The 1-based CSV line number.
    /// * `e`    — The [`csv::Error`] describing the parse failure.
    ///
    /// # Returns
    /// `"[WARN]    Line {line}: skipping invalid row: {e}"`
    pub fn csv_invalid_row(line: usize, e: csv::Error) -> String {
        format!("[WARN]    Line {}: skipping invalid row: {}", line, e)
    }
}
