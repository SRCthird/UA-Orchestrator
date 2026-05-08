// // Copyright 2026 Merck KGaA, Darmstadt, Germany and/or its affiliates.
// // All rights reserved

use opcua_client::prelude::*;
use std::fs;
use serde::Deserialize;
use crate::globals::Globals;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub server_url: String,
    pub server_security_policy: String,
    pub server_security_mode: String,
    pub username: String,
    pub password: String,
}

impl Config {
    pub fn load(path: &str) -> Self {
        let content = fs::read_to_string(path)
            .unwrap_or_else(|e| panic!("{}", Globals::config_failed_read(path, e)));

        toml::from_str(&content)
            .unwrap_or_else(|e| panic!("{}", Globals::config_failed_parse(path, e)))
    }

    pub fn security_policy(&self) -> SecurityPolicy {
        match self.server_security_policy.as_str() {
            "None" => SecurityPolicy::None,
            "Basic128Rsa15" => SecurityPolicy::Basic128Rsa15,
            "Basic256" => SecurityPolicy::Basic256,
            "Basic256Sha256" => SecurityPolicy::Basic256Sha256,
            other => panic!("{}", Globals::unsupported_security_policy(other)),
        }
    }

    pub fn security_mode(&self) -> MessageSecurityMode {
        match self.server_security_mode.as_str() {
            "None" => MessageSecurityMode::None,
            "Sign" => MessageSecurityMode::Sign,
            "SignAndEncrypt" => MessageSecurityMode::SignAndEncrypt,
            other => panic!("{}", Globals::unsupported_security_mode(other)),
        }
    }
}
