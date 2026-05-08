// // Copyright 2026 Merck KGaA, Darmstadt, Germany and/or its affiliates.
// // All rights reserved

use color_print::{cprintln, cformat};
use opcua_client::prelude::{NodeId, UAString, Variant};
use serde::Deserialize;
use std::thread;
use std::time::Duration;

use crate::globals::Globals;
use crate::opc_ua_client::OpcUaClient;
use crate::reader::InputReader;

#[derive(Debug, Deserialize, PartialEq)]
pub struct CsvRow {
    pub action: String,
    pub tag: String,
    pub value: Option<String>,
    pub sleep: u64,
}

pub fn parse_variant(s: &str) -> Variant {
    let lower = s.trim().to_lowercase();

    if lower == "true" {
        return Variant::Boolean(true);
    }
    if lower == "false" {
        return Variant::Boolean(false);
    }
    if let Ok(i) = s.trim().parse::<i64>() {
        return Variant::Int64(i);
    }
    if let Ok(f) = s.trim().parse::<f64>() {
        return Variant::Double(f);
    }
    Variant::String(UAString::from(s.trim()))
}

pub fn process_row(
    row: &CsvRow,
    line: usize,
    client: &mut impl OpcUaClient,
    reader: &mut impl InputReader,
) {
    let node_id = NodeId::new(2, row.tag.clone());

    match row.action.trim().to_lowercase().as_str() {
        s if s.starts_with("#") => { /* Document side Comment */ }
        "read" => match client.read(&node_id) {
            Some(v) => cprintln!(
                "<green>{}</>",
                Globals::csv_read_ok(&row.tag, &format!("{:?}", v))
            ),
            None => cprintln!("<yellow>{}</>", Globals::csv_read_no_value(&row.tag)),
        },
        "write" => match &row.value {
            Some(v_str) => {
                let variant = parse_variant(v_str);
                cprintln!(
                    "<bright-green>{}</>",
                    Globals::csv_write(&row.tag, &format!("{:?}", variant))
                );
                client.write(&node_id, variant);
            }
            None => {
                cprintln!(
                    "<bright-yellow>{}</>",
                    Globals::csv_write_missing_value(line, &row.tag)
                );
            }
        },
        "user_write" => {
            let raw = match &row.value {
                Some(v_str) => {
                    cprintln!(
                        "<bright-green>{}</>",
                        Globals::csv_user_write(&row.tag, v_str)
                    );
                    v_str.clone()
                }
                None => {
                    reader.read_line(
                        cformat!(
                            "<bright-green>{}</>",
                            Globals::csv_user_write_prompt(&row.tag)
                        )
                    )
                }
            };
            client.write(&node_id, parse_variant(&raw));
        }
        "comment" => {
            cprintln!("<white>{}</>", Globals::csv_comment(&row.tag));
        }
        "wait" => {
            reader.read_line(
                cformat!("<white>{}</>", Globals::csv_wait(&row.tag))
            );
        }
        "wait_until" => {
            if let Some(v_str) = &row.value {
                let target = parse_variant(v_str);
                let mut waiting_message_shown = false;

                loop {
                    match client.read(&node_id) {
                        Some(current) => {
                            if current == target {
                                cprintln!(
                                    "<green>{}</>", Globals::csv_wait_until_completed(&row.tag, &current)
                                );
                                break;
                            } else if !waiting_message_shown {
                                cprintln!(
                                    "<white>{}</>", Globals::csv_wait_until(&row.tag, &current, &target)
                                );
                                waiting_message_shown = true;
                            }
                        }
                        None => {
                            if !waiting_message_shown {
                                cprintln!("<bright-yellow>{}</>",
                                    Globals::csv_write_missing_value(line, &row.tag));
                                waiting_message_shown = true;
                            }
                        }
                    }

                    std::thread::sleep(std::time::Duration::from_millis(row.sleep.max(1)));
                }
            } else {
                cprintln!("<bright-yellow>{}</>",
                    Globals::csv_write_missing_value(line, &row.tag));
            }
        }

        other => {
            cprintln!("<yellow>{}</>", Globals::csv_unknown_action(line, other));
        }
    }

    if row.sleep > 0 {
        thread::sleep(Duration::from_millis(row.sleep));
    }
}

pub fn run_csv(client: &mut impl OpcUaClient, reader: &mut impl InputReader, csv_path: &str) {
    let mut rdr = csv::ReaderBuilder::new()
        .trim(csv::Trim::All)
        .from_path(csv_path)
        .unwrap_or_else(|e| panic!("{}", Globals::csv_open_failed(csv_path, e)));

    for (line, result) in rdr.deserialize::<CsvRow>().enumerate() {
        match result {
            Ok(row) => process_row(&row, line + 2, client, reader),
            Err(e) => eprintln!("{}", Globals::csv_invalid_row(line + 2, e)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use opcua_client::prelude::{NodeId, Variant, UAString};
    use std::collections::HashMap;
    use crate::opc_ua_client::OpcUaClient;
    use crate::reader::InputReader;

    // ── Fakes ────────────────────────────────────────────────────────────────

    #[derive(Default)]
    struct FakeClient {
        pub store: HashMap<String, Variant>,
        pub writes: Vec<(NodeId, Variant)>,
    }

    impl OpcUaClient for FakeClient {
        fn read(&self, node_id: &NodeId) -> Option<Variant> {
            self.store.get(&node_id.to_string()).cloned()
        }
        fn write(&mut self, node_id: &NodeId, value: Variant) {
            self.writes.push((node_id.clone(), value));
        }
    }

    struct ScriptedReader {
        lines: Vec<String>,
    }

    impl ScriptedReader {
        fn new(lines: &[&str]) -> Self {
            Self { lines: lines.iter().rev().map(|s| s.to_string()).collect() }
        }
    }

    impl InputReader for ScriptedReader {
        fn read_line(&mut self, _prompt: String) -> String {
            self.lines.pop().unwrap_or_default()
        }
    }

    // ── parse_variant tests ───────────────────────────────────────────────────

    #[test]
    fn parse_bool_true()  { assert_eq!(parse_variant("true"),  Variant::Boolean(true));  }
    #[test]
    fn parse_bool_false() { assert_eq!(parse_variant("False"), Variant::Boolean(false)); }
    #[test]
    fn parse_int()        { assert_eq!(parse_variant("42"),    Variant::Int64(42));       }
    #[test]
    fn parse_float()      { assert_eq!(parse_variant("3.14"),  Variant::Double(3.14));    }
    #[test]
    fn parse_string()     {
        assert_eq!(parse_variant("hello"),
            Variant::String(UAString::from("hello")));
    }

    // ── process_row tests ────────────────────────────────────────────────────

    #[test]
    fn write_row_calls_client() {
        let mut client = FakeClient::default();
        let mut reader = ScriptedReader::new(&[]);

        let row = CsvRow {
            action: "write".into(),
            tag: "MyTag".into(),
            value: Some("99".into()),
            sleep: 0,
        };

        process_row(&row, 2, &mut client, &mut reader);

        assert_eq!(client.writes.len(), 1);
        assert_eq!(client.writes[0].1, Variant::Int64(99));
    }

    #[test]
    fn user_write_reads_from_reader() {
        let mut client = FakeClient::default();
        let mut reader = ScriptedReader::new(&["123"]);

        let row = CsvRow {
            action: "user_write".into(),
            tag: "MyTag".into(),
            value: None,
            sleep: 0,
        };

        process_row(&row, 2, &mut client, &mut reader);

        assert_eq!(client.writes[0].1, Variant::Int64(123));
    }

    #[test]
    fn read_row_with_no_value_does_not_write() {
        let mut client = FakeClient::default();
        let mut reader = ScriptedReader::new(&[]);

        let row = CsvRow {
            action: "read".into(),
            tag: "Missing".into(),
            value: None,
            sleep: 0,
        };

        process_row(&row, 2, &mut client, &mut reader);
        assert!(client.writes.is_empty());
    }
}

