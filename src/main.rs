// Copyright 2026 Merck KGaA, Darmstadt, Germany and/or its affiliates.
// All rights reserved

use color_print::cformat;

use ua_orchestrator::config::Config;
use ua_orchestrator::globals::Globals;
use ua_orchestrator::reader::StdinReader;
use ua_orchestrator::opc_ua_client::{OpcUaSession, LiveOpcUaClient};
use ua_orchestrator::actions;

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
    let config  = Config::load(Globals::config_file());

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
