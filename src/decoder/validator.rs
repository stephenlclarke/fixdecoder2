// SPDX-License-Identifier: AGPL-3.0-only
// SPDX-FileCopyrightText: 2025 Steve Clarke <stephenlclarke@mac.com> - https://xyzzy.tools

use crate::decoder::fixparser::{FieldValue, parse_fix};
use crate::decoder::tag_lookup::{FixTagLookup, MessageDef};
use chrono::{NaiveDate, NaiveDateTime, NaiveTime};
use once_cell::sync::Lazy;
use regex::Regex;
use std::collections::{HashMap, HashSet};

/// Validate a single FIX message string against the provided dictionary,
/// returning a list of human-readable errors (or empty when valid).
pub fn validate_fix_message(msg: &str, dict: &FixTagLookup) -> Vec<String> {
    let fields = parse_fix(msg);
    let (field_map, seen_tags, duplicates) = build_field_map(&fields);
    let mut errors = Vec::new();

    for dup in duplicates {
        errors.push(format!("Duplicate tag {} encountered", dup));
    }

    let (msg_type_errs, msg_def_opt) = validate_msg_type(&field_map, dict);
    errors.extend(msg_type_errs);
    let Some(msg_def) = msg_def_opt else {
        return errors;
    };

    errors.extend(validate_required_fields(
        &msg_def.required,
        &seen_tags,
        dict,
    ));
    errors.extend(validate_field_enums_and_types(&fields, dict));
    errors.extend(validate_field_ordering(&fields, &msg_def.field_order));
    errors.extend(validate_checksum_field(msg, &field_map));

    errors
}

fn build_field_map(fields: &[FieldValue]) -> (HashMap<u32, String>, HashSet<u32>, Vec<u32>) {
    let mut field_map = HashMap::new();
    let mut seen = HashSet::new();
    let mut duplicates = Vec::new();
    for field in fields {
        if !seen.insert(field.tag) {
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
) -> Vec<String> {
    let mut errors = Vec::new();
    for tag in required {
        if !seen_tags.contains(tag) {
            errors.push(format!(
                "Missing required tag {} ({})",
                tag,
                dict.field_name(*tag)
            ));
        }
    }
    errors
}

fn validate_field_enums_and_types(fields: &[FieldValue], dict: &FixTagLookup) -> Vec<String> {
    let mut errors = Vec::new();
    for field in fields {
        if let Some(enums) = dict.enums_for(field.tag)
            && !enums.contains_key(&field.value)
        {
            errors.push(format!(
                "Invalid enum value '{}' for tag {}",
                field.value, field.tag
            ));
        }

        if let Some(field_type) = dict.field_type(field.tag)
            && !is_valid_type(&field.value, field_type)
        {
            errors.push(format!(
                "Invalid type for tag {}: expected {}, got '{}'",
                field.tag, field_type, field.value
            ));
        }
    }
    errors
}

fn validate_field_ordering(fields: &[FieldValue], expected_order: &[u32]) -> Vec<String> {
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
                errors.push(format!("Tag {} out of order", field.tag));
            }
            last_index = idx;
        }
    }
    errors
}

fn validate_checksum_field(msg: &str, field_map: &HashMap<u32, String>) -> Vec<String> {
    let mut errors = Vec::new();
    match field_map.get(&10) {
        None => errors.push("Missing required checksum tag 10".to_string()),
        Some(value) => {
            let expected = format!("{:03}", calculate_checksum(msg));
            if &expected != value {
                errors.push(format!(
                    "Checksum mismatch: got {}, expected {}",
                    value, expected
                ));
            }
        }
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
