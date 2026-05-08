mod config;
mod globals;

use crate::globals::Globals;

fn main() {
    let config  = config::Config::load(Globals::config_file());
    println!("{:#?}", config);
}
