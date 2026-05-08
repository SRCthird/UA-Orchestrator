mod config;

fn main() {
    let config  = config::Config::load("Config.toml");
    println!("{:#?}", config);
}
