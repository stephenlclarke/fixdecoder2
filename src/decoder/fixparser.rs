// SPDX-License-Identifier: AGPL-3.0-only
// SPDX-FileCopyrightText: 2025 Steve Clarke <stephenlclarke@mac.com> - https://xyzzy.tools

const SOH: char = '\u{0001}';

/// Parsed representation of a single FIX tag/value pair.
#[derive(Debug, Clone)]
pub struct FieldValue {
    pub tag: u32,
    pub value: String,
}

/// Split a FIX message string into ordered tag/value pairs, skipping fragments without `=`.
pub fn parse_fix(msg: &str) -> Vec<FieldValue> {
    if !msg.contains(SOH) {
        return Vec::new();
    }

    msg.split(SOH)
        .filter_map(|fragment| {
            if fragment.is_empty() {
                return None;
            }
            let (tag, value) = fragment.split_once('=')?;
            let tag_num: u32 = tag.parse().ok()?;
            Some(FieldValue {
                tag: tag_num,
                value: value.to_string(),
            })
        })
        .collect()
}
