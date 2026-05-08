#![allow(dead_code)]

pub struct Globals;

impl Globals {
    pub fn new() -> Self {
        Globals
    }

    pub fn config_file() -> &'static str {
        "Config.toml"
    }
    pub fn config_failed_read(path: &str, e: std::io::Error) -> String {
        format!("Failed to read {}: {}", path, e)
    }
    pub fn config_failed_parse(path: &str, e: toml::de::Error) -> String {
        format!("Failed to parse {}: {}", path, e)
    }

    pub fn unsupported_security_mode(other: &str) -> String {
        format!("Unsupported security mode: {}", other)
    }
    pub fn unsupported_security_policy(other: &str) -> String {
        format!("Unsupported security policy: {}", other)
    }


}
