mod config;
mod globals;
mod reader;
mod opc_ua_client;
mod actions;

use color_print::cformat;

use crate::globals::Globals;
use crate::reader::StdinReader;
use crate::opc_ua_client::{OpcUaSession,LiveOpcUaClient};

fn main() {
    let config  = config::Config::load(Globals::config_file());
    println!("{:#?}", config);

    let mut stdreader = StdinReader;
    let args: Vec<String> = std::env::args().collect();

    let csv_path = if args.len() > 1 {
        args[1].clone()
    } else {
        let prompt = cformat!("<white>{}</>", Globals::csv_request_path());
        stdreader.read_line(prompt)
    };
    println!("{}", csv_path);

    let session = OpcUaSession::new(&config);
    let mut client = LiveOpcUaClient { session: &session };

    actions::process_row(&actions::CsvRow {
        action: String::from("read"),
        tag: String::from("Channel1.Device1.CycleStart"),
        value: Some(String::from("true")),
        sleep: 0,
    }, 0, &mut client, &mut stdreader);

}
