// SPDX-License-Identifier: AGPL-3.0-only
// SPDX-FileCopyrightText: 2025 Steve Clarke <stephenlclarke@mac.com> - https://xyzzy.tools

use crate::decoder::fixparser::{FieldValue, parse_fix};
use crate::decoder::tag_lookup::{FixTagLookup, MessageDef};
use chrono::{NaiveDate, NaiveDateTime, NaiveTime};
use once_cell::sync::Lazy;
use regex::Regex;
use std::collections::{HashMap, HashSet};

#[derive(Debug, Default)]
pub struct ValidationReport {
    pub errors: Vec<String>,
    pub tag_errors: HashMap<u32, Vec<String>>,
}

impl ValidationReport {
    pub fn is_clean(&self) -> bool {
        self.errors.is_empty()
    }
}

/// Validate a single FIX message string against the provided dictionary,
/// returning a list of human-readable errors (or empty when valid).
pub fn validate_fix_message(msg: &str, dict: &FixTagLookup) -> ValidationReport {
    let fields = parse_fix(msg);
    let (field_map, seen_tags, duplicates) = build_field_map(&fields, dict);
    let mut errors = Vec::new();
    let mut tag_errors: HashMap<u32, Vec<String>> = HashMap::new();

    for dup in duplicates {
        let err = format!("Duplicate tag {} encountered", dup);
        errors.push(err.clone());
        tag_errors.entry(dup).or_default().push(err);
    }

    let (msg_type_errs, msg_def_opt) = validate_msg_type(&field_map, dict);
    errors.extend(msg_type_errs);
    let Some(msg_def) = msg_def_opt else {
        return ValidationReport { errors, tag_errors };
    };

    errors.extend(validate_required_fields(
        &msg_def.required,
        &seen_tags,
        dict,
        &mut tag_errors,
    ));
    errors.extend(validate_body_length(msg, &field_map, &mut tag_errors));
    errors.extend(validate_field_enums_and_types(
        &fields,
        dict,
        &mut tag_errors,
    ));
    errors.extend(validate_field_ordering(
        &fields,
        &msg_def.field_order,
        &mut tag_errors,
    ));
    errors.extend(validate_checksum_field(msg, &field_map, &mut tag_errors));

    ValidationReport { errors, tag_errors }
}

fn build_field_map(
    fields: &[FieldValue],
    dict: &FixTagLookup,
) -> (HashMap<u32, String>, HashSet<u32>, Vec<u32>) {
    let mut field_map = HashMap::new();
    let mut seen = HashSet::new();
    let mut duplicates = Vec::new();
    for field in fields {
        if !seen.insert(field.tag) && !dict.is_repeatable(field.tag) {
            duplicates.push(field.tag);
        }
        field_map.insert(field.tag, field.value.clone());
    }
    (field_map, seen, duplicates)
}

fn validate_msg_type<'a>(
    field_map: &HashMap<u32, String>,
    dict: &'a FixTagLookup,
) -> (Vec<String>, Option<&'a MessageDef>) {
    match field_map.get(&35) {
        None => (vec!["Missing required tag 35 (MsgType)".to_string()], None),
        Some(msg_type) => match dict.message_def(msg_type) {
            Some(def) => (Vec::new(), Some(def)),
            None => (vec![format!("Unknown MsgType: {}", msg_type)], None),
        },
    }
}

fn validate_required_fields(
    required: &[u32],
    seen_tags: &HashSet<u32>,
    dict: &FixTagLookup,
    tag_errors: &mut HashMap<u32, Vec<String>>,
) -> Vec<String> {
    let mut errors = Vec::new();
    for tag in required {
        if !seen_tags.contains(tag) {
            let err = format!("Missing required tag {} ({})", tag, dict.field_name(*tag));
            errors.push(err.clone());
            tag_errors.entry(*tag).or_default().push(err);
        }
    }
    errors
}

fn validate_field_enums_and_types(
    fields: &[FieldValue],
    dict: &FixTagLookup,
    tag_errors: &mut HashMap<u32, Vec<String>>,
) -> Vec<String> {
    let mut errors = Vec::new();
    for field in fields {
        if let Some(enums) = dict.enums_for(field.tag)
            && !enums.contains_key(&field.value)
        {
            let err = format!("Invalid enum value '{}'", field.value);
            errors.push(err.clone());
            tag_errors.entry(field.tag).or_default().push(err);
        }

        if let Some(field_type) = dict.field_type(field.tag)
            && !is_valid_type(&field.value, field_type)
        {
            let err = format!(
                "Invalid type: expected {}, got '{}'",
                field_type, field.value
            );
            errors.push(err.clone());
            tag_errors.entry(field.tag).or_default().push(err);
        }
    }
    errors
}

fn validate_field_ordering(
    fields: &[FieldValue],
    expected_order: &[u32],
    tag_errors: &mut HashMap<u32, Vec<String>>,
) -> Vec<String> {
    let mut order_index = HashMap::new();
    for (idx, tag) in expected_order.iter().enumerate() {
        order_index.insert(*tag, idx);
    }

    let mut errors = Vec::new();
    let mut last_index = -1isize;
    for field in fields {
        if let Some(&idx) = order_index.get(&field.tag) {
            let idx = idx as isize;
            if idx < last_index {
                let err = format!("Tag {} out of order", field.tag);
                errors.push(err.clone());
                tag_errors.entry(field.tag).or_default().push(err);
            }
            last_index = idx;
        }
    }
    errors
}

fn validate_checksum_field(
    msg: &str,
    field_map: &HashMap<u32, String>,
    tag_errors: &mut HashMap<u32, Vec<String>>,
) -> Vec<String> {
    let mut errors = Vec::new();
    match field_map.get(&10) {
        None => errors.push("Missing required checksum tag 10".to_string()),
        Some(value) => {
            let expected = format!("{:03}", calculate_checksum(msg));
            if &expected != value {
                let err = format!("Checksum mismatch: got {}, expected {}", value, expected);
                errors.push(err.clone());
                tag_errors.entry(10).or_default().push(err);
            }
        }
    }
    errors
}

fn validate_body_length(
    msg: &str,
    field_map: &HashMap<u32, String>,
    tag_errors: &mut HashMap<u32, Vec<String>>,
) -> Vec<String> {
    let mut errors = Vec::new();
    match field_map.get(&9) {
        None => errors.push("Missing required BodyLength tag 9".to_string()),
        Some(value) => match value.parse::<usize>() {
            Err(_) => errors.push(format!("Invalid BodyLength value '{}'", value)),
            Ok(declared) => match compute_actual_body_length(msg) {
                None => errors.push("Unable to compute BodyLength from message".to_string()),
                Some(actual) if declared != actual => {
                    let err = format!("BodyLength mismatch: got {}, expected {}", declared, actual);
                    tag_errors.entry(9).or_default().push(err.clone());
                    errors.push(err);
                }
                _ => {}
            },
        },
    }
    errors
}

pub fn calculate_checksum(msg: &str) -> i32 {
    const SOH: &str = "\u{0001}";
    if let Some(idx) = msg.rfind(&(SOH.to_string() + "10=")) {
        let fragment = &msg[..idx + 1];
        let sum: i32 = fragment.bytes().map(|b| b as i32).sum();
        sum % 256
    } else {
        -1
    }
}

fn is_valid_type(value: &str, field_type: &str) -> bool {
    match field_type.to_ascii_uppercase().as_str() {
        "INT" | "LENGTH" | "NUMINGROUP" | "SEQNUM" | "DAYOFMONTH" => value.parse::<i64>().is_ok(),
        "FLOAT" | "QTY" | "PRICE" | "PRICEOFFSET" | "AMT" | "PERCENTAGE" => {
            value.parse::<f64>().is_ok()
        }
        "BOOLEAN" => value == "Y" || value == "N",
        "CHAR" => value.chars().count() == 1,
        "STRING"
        | "DATA"
        | "CURRENCY"
        | "EXCHANGE"
        | "COUNTRY"
        | "MULTIPLEVALUESTRING"
        | "MULTIPLESTRINGVALUE" => true,
        "UTCTIMESTAMP" => is_valid_timestamp(value),
        "UTCDATEONLY" => NaiveDate::parse_from_str(value, "%Y%m%d").is_ok(),
        "UTCTIMEONLY" => ["%H:%M", "%H:%M:%S", "%H:%M:%S%.3f"]
            .iter()
            .any(|fmt| NaiveTime::parse_from_str(value, fmt).is_ok()),
        "MONTHYEAR" => MONTH_YEAR_REGEX.is_match(value),
        _ => true,
    }
}

fn is_valid_timestamp(value: &str) -> bool {
    ["%Y%m%d-%H:%M:%S", "%Y%m%d-%H:%M:%S%.3f"]
        .iter()
        .any(|fmt| NaiveDateTime::parse_from_str(value, fmt).is_ok())
}

static MONTH_YEAR_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^\d{6}(\d{2}|(-\d{1,2})|(-?w[1-5]))?$").expect("valid regex"));

fn compute_actual_body_length(msg: &str) -> Option<usize> {
    const SOH: u8 = 0x01;
    let bytes = msg.as_bytes();

    // find start of 9= field
    let mut len_pos = None;
    for i in 0..bytes.len().saturating_sub(1) {
        if bytes[i] == b'9' && bytes[i + 1] == b'=' {
            len_pos = Some(i);
            break;
        }
    }
    let len_pos = len_pos?;

    // find delimiter after 9= value
    let mut body_start = None;
    for (i, byte) in bytes.iter().enumerate().skip(len_pos) {
        if *byte == SOH {
            body_start = Some(i + 1);
            break;
        }
    }
    let body_start = body_start?;

    // find last occurrence of SOH10=
    let mut checksum_start = None;
    for i in (0..bytes.len().saturating_sub(3)).rev() {
        if bytes[i] == SOH && bytes[i + 1] == b'1' && bytes[i + 2] == b'0' && bytes[i + 3] == b'=' {
            checksum_start = Some(i);
            break;
        }
    }
    let checksum_start = checksum_start?;

    if checksum_start >= body_start {
        Some(checksum_start - body_start + 1)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::decoder::schema::{
        ComponentContainer, ComponentDef, Field, FieldContainer, FieldRef, FixDictionary, GroupDef,
        Message, MessageContainer, ValuesWrapper,
    };

    const SOH: &str = "\u{0001}";

    fn field(name: &str, number: u32, field_type: &str) -> Field {
        Field {
            name: name.to_string(),
            number,
            field_type: field_type.to_string(),
            values: Vec::new(),
            values_wrapper: ValuesWrapper::default(),
        }
    }

    fn test_lookup() -> FixTagLookup {
        let dict = FixDictionary {
            typ: "FIX".to_string(),
            major: "4".to_string(),
            minor: "4".to_string(),
            service_pack: None,
            fields: FieldContainer {
                items: vec![
                    field("BeginString", 8, "STRING"),
                    field("BodyLength", 9, "LENGTH"),
                    field("MsgType", 35, "STRING"),
                    field("CheckSum", 10, "STRING"),
                    field("NoItems", 100, "NUMINGROUP"),
                    field("ItemValue", 101, "STRING"),
                ],
            },
            messages: MessageContainer {
                items: vec![Message {
                    name: "Test".to_string(),
                    msg_type: "Z".to_string(),
                    msg_cat: "app".to_string(),
                    fields: vec![FieldRef {
                        name: "NoItems".to_string(),
                        required: Some("Y".to_string()),
                    }],
                    groups: vec![GroupDef {
                        name: "NoItems".to_string(),
                        required: Some("Y".to_string()),
                        fields: vec![FieldRef {
                            name: "ItemValue".to_string(),
                            required: Some("N".to_string()),
                        }],
                        groups: Vec::new(),
                        components: Vec::new(),
                    }],
                    components: Vec::new(),
                }],
            },
            components: ComponentContainer { items: Vec::new() },
            header: ComponentDef {
                name: String::new(),
                fields: vec![
                    FieldRef {
                        name: "BeginString".to_string(),
                        required: Some("Y".to_string()),
                    },
                    FieldRef {
                        name: "BodyLength".to_string(),
                        required: Some("Y".to_string()),
                    },
                    FieldRef {
                        name: "MsgType".to_string(),
                        required: Some("Y".to_string()),
                    },
                ],
                groups: Vec::new(),
                components: Vec::new(),
            },
            trailer: ComponentDef {
                name: String::new(),
                fields: vec![FieldRef {
                    name: "CheckSum".to_string(),
                    required: Some("Y".to_string()),
                }],
                groups: Vec::new(),
                components: Vec::new(),
            },
        };

        FixTagLookup::from_dictionary(&dict)
    }

    fn build_message(fields: &[(u32, &str)], declared_body_len: Option<usize>) -> String {
        let mut body = String::new();
        for (tag, value) in fields {
            body.push_str(&format!("{tag}={value}{SOH}"));
        }
        let body_len = declared_body_len.unwrap_or(body.len());
        let mut msg = format!("8=FIX.4.4{SOH}9={:03}{SOH}{}", body_len, body);
        let checksum = calculate_checksum(&format!("{msg}10=000{SOH}"));
        msg.push_str(&format!("10={:03}{SOH}", checksum));
        msg
    }

    #[test]
    fn allows_repeating_group_tags() {
        let dict = test_lookup();
        let msg = build_message(
            &[(35, "Z"), (100, "2"), (101, "ALPHA"), (101, "BETA")],
            None,
        );
        let errors = validate_fix_message(&msg, &dict);
        assert!(
            errors.is_clean(),
            "expected no errors for valid repeating group message: {:?}",
            errors.errors
        );
    }

    #[test]
    fn detects_body_length_mismatch() {
        let dict = test_lookup();
        let msg = build_message(&[(35, "Z"), (100, "1"), (101, "ONLY")], Some(999));
        let errors = validate_fix_message(&msg, &dict);
        assert!(
            errors
                .errors
                .iter()
                .any(|e| e.contains("BodyLength mismatch")),
            "expected body length error, got {:?}",
            errors.errors
        );
    }

    #[test]
    fn detects_checksum_mismatch() {
        let dict = test_lookup();
        let mut msg = build_message(&[(35, "Z"), (100, "1"), (101, "ONLY")], None);
        // Replace checksum with an incorrect value while keeping length intact.
        if let Some(pos) = msg.rfind("10=") {
            msg.truncate(pos + 3);
            msg.push_str("999\u{0001}");
        }
        let errors = validate_fix_message(&msg, &dict);
        assert!(
            errors
                .errors
                .iter()
                .any(|e| e.contains("Checksum mismatch")),
            "expected checksum mismatch, got {:?}",
            errors.errors
        );
    }
}
