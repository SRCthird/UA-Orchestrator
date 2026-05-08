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
