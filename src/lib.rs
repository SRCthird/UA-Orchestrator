// Copyright 2026 Merck KGaA, Darmstadt, Germany and/or its affiliates.
// All rights reserved

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
//! | `logger`          | Creates a logger wrapper that logs all actions            |
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

pub mod actions;
pub mod config;
pub mod globals;
pub mod opc_ua_client;
pub mod reader;
pub mod logger;
