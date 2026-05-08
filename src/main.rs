mod config;
mod globals;
mod reader;

use color_print::cformat;

use crate::globals::Globals;
use crate::reader::StdinReader;

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

}
