#![allow(dead_code)]

use opcua_client::prelude::{StatusCode,Variant};

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

    pub fn csv_request_path() -> &'static str {
        "[PATH]    Please provide path to csv <= "
    }

    pub fn app_name(user: &str) -> String {
        format!("UAOrchestrator@{user}")
    }
    pub fn app_uri(hostname: &str) -> String {
        format!("urn:{hostname}:UAOrchestrator")
    }
    pub fn app_user_msg(user: &str) -> String {
        format!("Executing User   : {}", user)
    }
    pub fn app_name_msg(app_name: &str) -> String {
        format!("Application Name : {}", app_name)
    }
    pub fn app_uri_msg(app_uri: &str) -> String {
        format!("Application URI  : {}", app_uri)
    }

    pub fn user_cert(user: &str) -> String {
        format!("own/cert_{}.der", user)
    }
    pub fn user_key(user: &str) -> String {
        format!("private/key_{}.pem", user)
    }

    pub fn endpoint_error() -> &'static str {
        "Could not find matching endpoint"
    }
    pub fn endpoint_connecting(endpoint: &str) -> String {
        format!("Connecting to    : {}", endpoint)
    }

    pub fn read_no_value(index: usize, status: StatusCode) -> String {
        format!("[WARN]    Node {}: no value, status = {}", index, status)
    }
    pub fn read_error(e: StatusCode) -> String {
        format!("[ERR]     Read failed: {}", e)
    }
    pub fn write_status(index: usize, status: StatusCode) -> String {
        format!("[WARN]    Node {}: write status = {}", index, status)
    }
    pub fn write_error(e: StatusCode) -> String {
        format!("[WARN]    Write failed: {}", e)
    }

    pub fn csv_read_ok(tag: &str, value: &str) -> String {
        format!("[READ]    {} => {}", tag, value)
    }
    pub fn csv_read_no_value(tag: &str) -> String {
        format!("[READ]    {} => no value", tag)
    }
    pub fn csv_write(tag: &str, value: &str) -> String {
        format!("[WRITE]   {} <= {}", tag, value)
    }
    pub fn csv_write_missing_value(line: usize, tag: &str) -> String {
        format!("[WRITE]   Line {}: missing value for write on '{}'", line, tag)
    }
    pub fn csv_user_write(tag: &str, value: &str) -> String {
        format!("[UWRITE]  {} <= {}", tag, value)
    }
    pub fn csv_user_write_prompt(tag: &str) -> String {
        format!("[UWRITE]  {} <= ", tag)
    }
    pub fn csv_comment(tag: &str) -> String {
        format!("[COMMENT] {}", tag)
    }
    pub fn csv_unknown_action(line: usize, action: &str) -> String {
        format!("[WARN]    Line {}: unknown action '{}', skipping", line, action)
    }
    pub fn csv_wait(tag: &str) -> String {
        format!("[WAIT]    {}", tag)
    }
    pub fn csv_wait_until(tag: &str, target: &Variant, current: &Variant) -> String {
        format!("[WAITFOR] {}:{:?} == {:?}", tag, current, target)
    }
    pub fn csv_wait_until_completed(tag: &str, current: &Variant) -> String {
        format!("[WAITFOR] Satisfied: {} == {:?}", tag, current)
    }

}
