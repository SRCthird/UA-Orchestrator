// Copyright 2026 Merck KGaA, Darmstadt, Germany and/or its affiliates.
// All rights reserved

//! # Configuration
//!
//! This module owns the [`Config`] struct, which is deserialised from a
//! [TOML](https://toml.io) file at runtime. It also provides conversion
//! helpers that translate the human-readable strings stored in the TOML file
//! into the strongly-typed OPC UA enumerations required by `opcua_client`.
//!
//! ## TOML File Format
//!
//! | Key                        | Type   | Description                                               |
//! |----------------------------|--------|-----------------------------------------------------------|
//! | `server_url`               | String | Full OPC UA endpoint URL, e.g. `opc.tcp://host:4840`      |
//! | `server_security_policy`   | String | One of `None`, `Basic128Rsa15`, `Basic256`, `Basic256Sha256` |
//! | `server_security_mode`     | String | One of `None`, `Sign`, `SignAndEncrypt`                   |
//! | `username`                 | String | OPC UA session username                                   |
//! | `password`                 | String | OPC UA session password                                   |
//!
//! ## Example TOML
//!
//! ```toml
//! server_url              = "opc.tcp://localhost:4840"
//! server_security_policy  = "Basic256Sha256"
//! server_security_mode    = "SignAndEncrypt"
//! username                = "operator"
//! password                = "s3cr3t"
//! ```
//!
//! ## Panics
//!
//! Every public entry-point in this module panics on misconfiguration rather
//! than returning a `Result`. This is intentional: a missing or malformed
//! config file, or an unrecognised security string, is considered a
//! non-recoverable start-up error.

use opcua_client::prelude::*;
use std::fs;
use serde::Deserialize;
use crate::globals::Globals;

/// Runtime configuration for an OPC UA client session.
///
/// All fields are read directly from a TOML file via [`Config::load`] and
/// are plain [`String`] values. Use [`Config::security_policy`] and
/// [`Config::security_mode`] to obtain the corresponding strongly-typed OPC UA
/// enum values when opening a session.
///
/// # Example
///
/// ```rust,ignore
/// let cfg = Config::load("config.toml");
/// println!("Connecting to {}", cfg.server_url);
/// let session = client
///     .connect(cfg.server_url, cfg.security_policy(), cfg.security_mode(),
///              cfg.username, cfg.password);
/// ```
#[derive(Debug, Deserialize)]
pub struct Config {
    /// Full OPC UA endpoint URL.
    ///
    /// Must include scheme, host, and port — e.g.
    /// `opc.tcp://192.168.1.10:4840/endpoint`.
    pub server_url: String,

    /// OPC UA security policy as a string.
    ///
    /// Accepted values: `"None"`, `"Basic128Rsa15"`, `"Basic256"`,
    /// `"Basic256Sha256"`. Any other value causes a panic at runtime.
    /// Convert to the enum type via [`Config::security_policy`].
    pub server_security_policy: String,

    /// OPC UA message security mode as a string.
    ///
    /// Accepted values: `"None"`, `"Sign"`, `"SignAndEncrypt"`. Any other
    /// value causes a panic at runtime.
    /// Convert to the enum type via [`Config::security_mode`].
    pub server_security_mode: String,

    /// OPC UA session username.
    pub username: String,

    /// OPC UA session password.
    pub password: String,
}

impl Config {
    /// Loads and deserialises a [`Config`] from a TOML file.
    ///
    /// Reads the file at `path` into a string and then parses it with
    /// [`toml::from_str`]. Both steps panic with descriptive messages on
    /// failure so that misconfiguration is caught immediately at start-up.
    ///
    /// # Arguments
    ///
    /// * `path` - Filesystem path to the TOML configuration file.
    ///
    /// # Panics
    ///
    /// - If the file cannot be read (e.g. not found, permission denied).
    /// - If the file content is not valid TOML or is missing required keys.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// let config = Config::load("config.toml");
    /// ```
    pub fn load(path: &str) -> Self {
        let content = fs::read_to_string(path)
            .unwrap_or_else(|e| panic!("{}", Globals::config_failed_read(path, e)));

        toml::from_str(&content)
            .unwrap_or_else(|e| panic!("{}", Globals::config_failed_parse(path, e)))
    }

    /// Returns the [`SecurityPolicy`] enum value for this configuration.
    ///
    /// Maps [`Config::server_security_policy`] to the corresponding
    /// `opcua_client` enum variant.
    ///
    /// # Supported values
    ///
    /// | TOML string        | Enum variant                    |
    /// |--------------------|---------------------------------|
    /// | `"None"`           | [`SecurityPolicy::None`]        |
    /// | `"Basic128Rsa15"` | [`SecurityPolicy::Basic128Rsa15`] |
    /// | `"Basic256"`       | [`SecurityPolicy::Basic256`]    |
    /// | `"Basic256Sha256"` | [`SecurityPolicy::Basic256Sha256`] |
    ///
    /// # Panics
    ///
    /// Panics if the string does not match any supported variant.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// let policy = config.security_policy(); // e.g. SecurityPolicy::Basic256Sha256
    /// ```
    pub fn security_policy(&self) -> SecurityPolicy {
        match self.server_security_policy.as_str() {
            "None" => SecurityPolicy::None,
            "Basic128Rsa15" => SecurityPolicy::Basic128Rsa15,
            "Basic256" => SecurityPolicy::Basic256,
            "Basic256Sha256" => SecurityPolicy::Basic256Sha256,
            other => panic!("{}", Globals::unsupported_security_policy(other)),
        }
    }

    /// Returns the [`MessageSecurityMode`] enum value for this configuration.
    ///
    /// Maps [`Config::server_security_mode`] to the corresponding
    /// `opcua_client` enum variant.
    ///
    /// # Supported values
    ///
    /// | TOML string          | Enum variant                          |
    /// |----------------------|---------------------------------------|
    /// | `"None"`             | [`MessageSecurityMode::None`]         |
    /// | `"Sign"`             | [`MessageSecurityMode::Sign`]         |
    /// | `"SignAndEncrypt"`   | [`MessageSecurityMode::SignAndEncrypt`] |
    ///
    /// # Panics
    ///
    /// Panics if the string does not match any supported variant.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// let mode = config.security_mode(); // e.g. MessageSecurityMode::SignAndEncrypt
    /// ```
    pub fn security_mode(&self) -> MessageSecurityMode {
        match self.server_security_mode.as_str() {
            "None"           => MessageSecurityMode::None,
            "Sign"           => MessageSecurityMode::Sign,
            "SignAndEncrypt" => MessageSecurityMode::SignAndEncrypt,
            other => panic!("{}", Globals::unsupported_security_mode(other)),
        }
    }
}
