// Copyright 2026 Merck KGaA, Darmstadt, Germany and/or its affiliates.
// All rights reserved

use color_print::cformat;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use ua_orchestrator::config::Config;
use ua_orchestrator::globals::Globals;
use ua_orchestrator::logger::Logger;
use ua_orchestrator::reader::StdinReader;
use ua_orchestrator::opc_ua_client::{OpcUaSession, LiveOpcUaClient};
use ua_orchestrator::actions;

/// Application entry point.
///
/// Executes the following steps in order:
///
/// 1. **Load configuration** from `Config.toml` via [`config::Config::load`].
/// 2. **Construct the reader** — wraps stdin with an optional log file taken
///    from `config.log_file`.
/// 3. **Resolve the CSV path** — uses `args[1]` if provided on the command
///    line, otherwise falls back to an interactive prompt via [`StdinReader`].
/// 4. **Open an OPC UA session** via [`OpcUaSession::new`], authenticating
///    against the server defined in the loaded config.
/// 5. **Wrap the session** in a [`LiveOpcUaClient`] that implements the
///    [`opc_ua_client::OpcUaClient`] trait expected by the action runner.
/// 6. **Execute the CSV script** by calling [`actions::run_csv`], which reads
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
    let config = Config::load(Globals::config_file());

    let log_path = config.log_file.as_deref().map(PathBuf::from);
    let logger   = Arc::new(Mutex::new(Logger::new(log_path)));

    let mut stdreader = StdinReader::new(Arc::clone(&logger));

    let args: Vec<String> = std::env::args().collect();
    let csv_path = if args.len() > 1 {
        args[1].clone()
    } else {
        let prompt = cformat!("<white>{}</>", Globals::csv_request_path());
        stdreader.read_line(prompt)
    };

    
    let session    = OpcUaSession::new(&config, Arc::clone(&logger));
    let mut client = LiveOpcUaClient { session: &session };

    actions::run_csv(&mut client, &mut stdreader, &csv_path, &logger);
}
