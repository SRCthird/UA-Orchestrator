// // Copyright 2026 Merck KGaA, Darmstadt, Germany and/or its affiliates.
// // All rights reserved

//! # UAOrchestrator — Application Entry Point
//!
//! This is the top-level binary crate root for **UAOrchestrator**. It wires
//! together the configuration loader, OPC UA session, console reader, and CSV
//! action runner into a single linear start-up sequence.
//!
//! ## Start-up Sequence
//!
//! ```text
//! 1. Load Config.toml          →  config::Config::load
//! 2. Resolve the CSV path      →  CLI arg[1]  OR  interactive prompt
//! 3. Establish OPC UA session  →  OpcUaSession::new
//! 4. Build the live client     →  LiveOpcUaClient { session }
//! 5. Execute the CSV script    →  actions::run_csv
//! ```
//!
//! ## Usage
//!
//! Supply the CSV path as a command-line argument:
//!
//! ```text
//! cargo run --release -- path/to/script.csv
//! ```
//!
//! Or run without arguments to be prompted interactively:
//!
//! ```text
//! cargo run --release
//! [PATH]    Please provide path to csv <=
//! ```
//!
//! ## Module Graph
//!
//! | Module            | Role                                                      |
//! |-------------------|-----------------------------------------------------------|
//! | `config`          | Loads and parses `Config.toml`                            |
//! | `globals`         | Centralised string and message constants                  |
//! | `reader`          | Abstracts console input (`StdinReader` / `InputReader`)   |
//! | `opc_ua_client`   | OPC UA session wrapper and client trait                   |
//! | `actions`         | CSV row parser and action dispatcher                      |
//!
//! ## Configuration
//!
//! `Config.toml` is always read from the **current working directory**.
//! See [`config::Config`] for the full list of required TOML keys.
//!
//! ## Panics
//!
//! The process will terminate with a descriptive panic message if:
//!
//! - `Config.toml` cannot be read or parsed.
//! - The OPC UA endpoint cannot be found or connected to.
//! - An unsupported `security_policy` or `security_mode` value is present in
//!   the config.

mod config;
mod globals;
mod reader;
mod opc_ua_client;
mod actions;

use color_print::cformat;

use crate::globals::Globals;
use crate::reader::StdinReader;
use crate::opc_ua_client::{OpcUaSession,LiveOpcUaClient};

/// Application entry point.
///
/// Executes the following steps in order:
///
/// 1. **Load configuration** from `Config.toml` via [`config::Config::load`].
/// 2. **Resolve the CSV path** — uses `args[1]` if provided on the command
///    line, otherwise falls back to an interactive prompt via [`StdinReader`].
/// 3. **Open an OPC UA session** via [`OpcUaSession::new`], authenticating
///    against the server defined in the loaded config.
/// 4. **Wrap the session** in a [`LiveOpcUaClient`] that implements the
///    [`opc_ua_client::OpcUaClient`] trait expected by the action runner.
/// 5. **Execute the CSV script** by calling [`actions::run_csv`], which reads
///    each row and dispatches the appropriate OPC UA operation.
///
/// # Panics
///
/// Panics with a descriptive message if:
///
/// - `Config.toml` cannot be read from disk or contains invalid TOML.
/// - The config specifies an unsupported security policy or security mode.
/// - No matching OPC UA endpoint can be found on the server.
fn main() {
    let config  = config::Config::load(Globals::config_file());

    let mut stdreader = StdinReader;
    let args: Vec<String> = std::env::args().collect();

    let csv_path = if args.len() > 1 {
        args[1].clone()
    } else {
        let prompt = cformat!("<white>{}</>", Globals::csv_request_path());
        stdreader.read_line(prompt)
    };

    let session = OpcUaSession::new(&config);
    let mut client = LiveOpcUaClient { session: &session };

    actions::run_csv(&mut client, &mut stdreader, &csv_path);
}
