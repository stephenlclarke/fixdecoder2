// SPDX-License-Identifier: AGPL-3.0-only
// SPDX-FileCopyrightText: 2025 Steve Clarke <stephenlclarke@mac.com> - https://xyzzy.tools

use crate::decoder::colours::{disable_colours, palette};
use crate::decoder::fixparser::{FieldValue, parse_fix};
use crate::decoder::tag_lookup::{FixTagLookup, MessageDef, load_dictionary};
use crate::decoder::validator;
use crate::fix;
use once_cell::sync::Lazy;
use regex::Regex;
use std::borrow::Cow;
use std::fs::File;
use std::io::{self, BufRead, BufReader, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use terminal_size::{Width, terminal_size};

static VALIDATION_ENABLED: AtomicBool = AtomicBool::new(false);

static FIX_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"8=FIX.*?10=\d{3}\u{0001}").expect("valid regex"));

/// Best-effort terminal width detection for separator rendering.
fn terminal_width() -> usize {
    if let Some((Width(w), _)) = terminal_size() {
        w as usize
    } else {
        80
    }
}

/// Enable or disable validation of FIX messages during prettification.
pub fn set_validation(enabled: bool) {
    VALIDATION_ENABLED.store(enabled, Ordering::Relaxed);
}

/// Returns whether validation is currently enabled for prettification.
pub fn validation_enabled() -> bool {
    VALIDATION_ENABLED.load(Ordering::Relaxed)
}

/// Render a single FIX message into a human-friendly string using the provided dictionary.
/// When a validation report is supplied, tag-level errors are annotated inline and missing
/// required fields are surfaced in the output.
pub fn prettify_with_report(
    msg: &str,
    dict: &FixTagLookup,
    report: Option<&validator::ValidationReport>,
) -> String {
    let colours = palette();
    let mut output = String::new();
    let fields = parse_fix(msg);
    let annotations = report.map(|r| &r.tag_errors);

    let mut tag_buckets = bucket_fields(&fields);
    let ordered_tags = build_tag_order(&fields, dict, annotations);

    for tag in ordered_tags {
        if let Some(bucket) = tag_buckets.get_mut(&tag) {
            while let Some(field) = bucket.pop_front() {
                write_field_line(&mut output, dict, field, annotations, &colours);
            }
        } else if let Some(errs) = annotations
            .and_then(|ann| ann.get(&tag))
            .filter(|errs| !errs.is_empty())
        {
            write_missing_line(&mut output, dict, tag, errs, &colours);
        }
    }

    // Emit any remaining fields that were not covered by ordered_tags.
    for bucket in tag_buckets.values_mut() {
        while let Some(field) = bucket.pop_front() {
            write_field_line(&mut output, dict, field, annotations, &colours);
        }
    }

    output
}

/// Bucket each field by tag so repeat occurrences can be emitted in order.
fn bucket_fields(
    fields: &[FieldValue],
) -> std::collections::HashMap<u32, std::collections::VecDeque<&FieldValue>> {
    use std::collections::{HashMap, VecDeque};
    let mut buckets: HashMap<u32, VecDeque<&FieldValue>> = HashMap::new();
    for field in fields {
        buckets.entry(field.tag).or_default().push_back(field);
    }
    buckets
}

/// Build the emission order of tags using the message definition when known, falling back
/// to a header-first order when MsgType is absent, and appending tags referenced in
/// validation annotations.
fn build_tag_order(
    fields: &[FieldValue],
    dict: &FixTagLookup,
    annotations: Option<&std::collections::HashMap<u32, Vec<String>>>,
) -> Vec<u32> {
    let msg_type = fields.iter().find(|f| f.tag == 35).map(|f| f.value.clone());
    let message_def: Option<MessageDef> = msg_type
        .as_deref()
        .and_then(|mt| dict.message_def(mt).cloned());

    let mut ordered_tags: Vec<u32> = match message_def.as_ref() {
        Some(def) => def.field_order.clone(),
        None => {
            // Best-effort ordering when MsgType is missing: emit header tags in canonical order first, then existing fields.
            let mut base = vec![8, 9, 35];
            for f in fields {
                if !base.contains(&f.tag) {
                    base.push(f.tag);
                }
            }
            base
        }
    };

    if let Some(ann) = annotations {
        let mut missing: Vec<u32> = ann
            .keys()
            .filter(|tag| !ordered_tags.contains(tag))
            .copied()
            .collect();
        missing.sort();
        ordered_tags.extend(missing);
    }

    ordered_tags
}

pub fn prettify_files(
    paths: &[String],
    out: &mut dyn Write,
    err_out: &mut dyn Write,
    obfuscator: &fix::Obfuscator,
    display_delimiter: char,
) -> i32 {
    let mut had_error = false;

    if paths.is_empty() {
        return handle_stdin(out, err_out, obfuscator, display_delimiter);
    }

    for path in paths {
        if path == "-" {
            if handle_stdin(out, err_out, obfuscator, display_delimiter) != 0 {
                had_error = true;
            }
            continue;
        }

        if handle_file(path, out, err_out, obfuscator, display_delimiter).is_err() {
            had_error = true;
        }
    }

    if had_error { 1 } else { 0 }
}

/// Write a single field line, including optional enum descriptions and validation errors.
fn write_field_line(
    output: &mut String,
    dict: &FixTagLookup,
    field: &crate::decoder::fixparser::FieldValue,
    annotations: Option<&std::collections::HashMap<u32, Vec<String>>>,
    colours: &crate::decoder::colours::ColourPalette,
) {
    let tag_errors: Option<&Vec<String>> = annotations.and_then(|ann| ann.get(&field.tag));
    let tag_colour = if tag_errors.is_some() {
        colours.error
    } else {
        colours.tag
    };
    let name = dict.field_name(field.tag);
    let desc = dict.enum_description(field.tag, &field.value);
    output.push_str(&format!(
        "    {}{:4}{} ({}{}{}): {}{}{}",
        tag_colour,
        field.tag,
        colours.reset,
        colours.name,
        name,
        colours.reset,
        colours.value,
        field.value,
        colours.reset
    ));

    if let Some(description) = desc {
        output.push_str(&format!(
            " ({}{}{})",
            colours.enumeration, description, colours.reset
        ));
    }

    if let Some(errs) = tag_errors {
        let msg = errs.join(", ");
        output.push_str(&format!("  {}{}{}", colours.error, msg, colours.reset));
    }

    output.push('\n');
}

/// Write a placeholder line for a missing field, showing validation errors when present.
fn write_missing_line(
    output: &mut String,
    dict: &FixTagLookup,
    tag: u32,
    errors: &[String],
    colours: &crate::decoder::colours::ColourPalette,
) {
    let name = dict.field_name(tag);
    let err_text = if errors.is_empty() {
        "Missing".to_string()
    } else {
        errors.join(", ")
    };
    output.push_str(&format!(
        "    {}{:4}{} ({}{}{}): {}{}{}\n",
        colours.error,
        tag,
        colours.reset,
        colours.name,
        name,
        colours.reset,
        colours.error,
        err_text,
        colours.reset
    ));
}

/// Handle decoding from stdin (used when no file paths are provided).
fn handle_stdin(
    out: &mut dyn Write,
    err_out: &mut dyn Write,
    obfuscator: &fix::Obfuscator,
    display_delimiter: char,
) -> i32 {
    obfuscator.reset();
    if !validation_enabled() {
        let _ = writeln!(out, "Processing: (stdin)\n");
    }
    if stream_reader(
        BufReader::new(io::stdin().lock()),
        out,
        obfuscator,
        display_delimiter,
    )
    .is_err()
    {
        let colours = palette();
        let _ = writeln!(
            err_out,
            "{}Error reading input{}",
            colours.error, colours.reset
        );
        return 1;
    }
    0
}

/// Handle decoding from a single file path, printing progress when validation is disabled.
fn handle_file(
    path: &str,
    out: &mut dyn Write,
    err_out: &mut dyn Write,
    obfuscator: &fix::Obfuscator,
    display_delimiter: char,
) -> io::Result<()> {
    obfuscator.reset();
    let colours = palette();
    if !validation_enabled() {
        let _ = writeln!(
            out,
            "Processing: {}{}{}\n",
            colours.file, path, colours.reset
        );
    }

    match File::open(path) {
        Ok(file) => {
            stream_reader(BufReader::new(file), out, obfuscator, display_delimiter)?;
        }
        Err(err) => {
            let _ = writeln!(
                err_out,
                "{}Cannot open file: {}{}",
                colours.error, err, colours.reset
            );
            return Err(err);
        }
    }
    Ok(())
}

/// Stream lines from a reader, emitting formatted FIX messages (and optionally validation output).
fn stream_reader<R: BufRead>(
    mut reader: R,
    out: &mut dyn Write,
    obfuscator: &fix::Obfuscator,
    display_delimiter: char,
) -> io::Result<()> {
    let mut line = String::new();
    let colours = palette();
    let separator = format!(
        "{}{}{}\n",
        colours.title,
        "=".repeat(terminal_width()),
        colours.reset
    );

    let mut line_number = 0usize;
    loop {
        line.clear();
        let bytes = reader.read_line(&mut line)?;
        if bytes == 0 {
            break;
        }
        line_number += 1;

        if line.ends_with('\n') {
            line.pop();
            if line.ends_with('\r') {
                line.pop();
            }
        }

        let processed = obfuscator.enabled_line(&line);
        handle_log_line(&processed, line_number, out, &separator, display_delimiter)?;
    }

    Ok(())
}

/// Process a single log line, extracting FIX messages and rendering prettified output.
fn handle_log_line(
    line: &str,
    line_number: usize,
    out: &mut dyn Write,
    separator: &str,
    display_delimiter: char,
) -> io::Result<()> {
    let matches = find_fix_message_indices(line);
    let colours = palette();

    if !validation_enabled() {
        if matches.is_empty() {
            writeln!(out, "{}{}{}", colours.line, line, colours.reset)?;
            return Ok(());
        }

        let (messages, coloured_line) =
            extract_messages_and_format(line, &matches, display_delimiter);
        write!(out, "{coloured_line}")?;
        write!(out, "{separator}")?;

        for msg in messages {
            process_fix_message(&msg, out, separator)?;
        }
        return Ok(());
    }

    if matches.is_empty() {
        return Ok(());
    }

    let mut invalid = Vec::new();
    for (start, end) in matches {
        let msg = &line[start..end];
        let dict = load_dictionary(msg);
        let report = validator::validate_fix_message(msg, &dict);
        if report.is_clean() {
            continue;
        }
        let pretty = prettify_with_report(msg, &dict, Some(&report));
        invalid.push((msg.to_string(), pretty, report.errors));
    }

    if invalid.is_empty() {
        return Ok(());
    }

    let display_line = apply_display_delimiter(line, display_delimiter);
    writeln!(
        out,
        "Line {}: {}{}{}",
        line_number, colours.line, display_line, colours.reset
    )?;

    for (_, pretty, _) in invalid {
        write!(out, "{pretty}")?;
        writeln!(out)?;
    }

    Ok(())
}

/// Locate FIX message spans within a line using a permissive regex.
fn find_fix_message_indices(line: &str) -> Vec<(usize, usize)> {
    FIX_REGEX
        .find_iter(line)
        .map(|m| (m.start(), m.end()))
        .collect()
}

/// Extract FIX messages from a line while also returning a coloured representation.
fn extract_messages_and_format(
    line: &str,
    matches: &[(usize, usize)],
    display_delimiter: char,
) -> (Vec<String>, String) {
    let colours = palette();
    let mut output = String::new();
    let mut fix_messages = Vec::new();
    let mut last = 0;

    for (start, end) in matches {
        output.push_str(colours.line);
        let before = &line[last..*start];
        let before_display = apply_display_delimiter(before, display_delimiter);
        output.push_str(&before_display);

        output.push_str(colours.message);
        let fix_segment = &line[*start..*end];
        let fix_display = apply_display_delimiter(fix_segment, display_delimiter);
        output.push_str(&fix_display);
        fix_messages.push(line[*start..*end].to_string());
        last = *end;
    }

    if last < line.len() {
        output.push_str(colours.line);
        let tail_display = apply_display_delimiter(&line[last..], display_delimiter);
        output.push_str(&tail_display);
    } else {
        output.push_str(colours.line);
    }

    output.push_str(colours.reset);
    output.push('\n');

    (fix_messages, output)
}

/// Replace SOH display delimiters for human-readable rendering without mutating inputs.
fn apply_display_delimiter<'a>(text: &'a str, delimiter: char) -> Cow<'a, str> {
    const SOH: char = '\u{0001}';
    if delimiter == SOH || !text.contains(SOH) {
        Cow::Borrowed(text)
    } else {
        let mut output = String::with_capacity(text.len());
        for ch in text.chars() {
            if ch == SOH {
                output.push(delimiter);
            } else {
                output.push(ch);
            }
        }
        Cow::Owned(output)
    }
}

/// Render a single FIX message (and validation errors when enabled) to the output stream.
fn process_fix_message(msg: &str, out: &mut dyn Write, separator: &str) -> io::Result<()> {
    let dict = load_dictionary(msg);
    let pretty = prettify_with_report(msg, &dict, None);
    write!(out, "{pretty}")?;

    if VALIDATION_ENABLED.load(Ordering::Relaxed) {
        let report = validator::validate_fix_message(msg, &dict);
        if !report.errors.is_empty() {
            let colours = palette();
            write!(out, "{separator}")?;
            for err in report.errors {
                writeln!(out, "{}== {}{}", colours.error, err, colours.reset)?;
            }
        }
    }

    write!(out, "{separator}")?;
    Ok(())
}

pub fn disable_output_colours() {
    disable_colours();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::decoder::tag_lookup::load_dictionary;
    use crate::decoder::validator;
    use crate::fix;
    use std::collections::HashMap;
    use std::io::Cursor;
    use std::sync::Mutex;

    const SOH: char = '\u{0001}';
    static TEST_GUARD: once_cell::sync::Lazy<Mutex<()>> =
        once_cell::sync::Lazy::new(|| Mutex::new(()));

    #[test]
    fn validation_only_outputs_invalid_messages() {
        let _lock = TEST_GUARD.lock().unwrap();
        set_validation(true);
        let obfuscator = fix::create_obfuscator(false);
        let body = format!("35=0{SOH}34=1{SOH}49=AAA{SOH}52=20240101-00:00:00{SOH}56=BBB{SOH}");
        let declared_len = body.len() + 1; // intentionally wrong
        let msg_without_checksum = format!("8=FIX.4.4{SOH}9={:03}{SOH}{}", declared_len, body);
        let checksum = validator::calculate_checksum(&format!("{msg_without_checksum}10=000{SOH}"));
        let msg = format!("{msg_without_checksum}10={checksum:03}{SOH}");
        let line = format!("{msg}\n");
        let mut out = Vec::new();
        stream_reader(
            BufReader::new(Cursor::new(line)),
            &mut out,
            &obfuscator,
            '|',
        )
        .unwrap();
        set_validation(false);

        let output = String::from_utf8(out).unwrap();
        assert!(
            output.contains("Line 1:"),
            "line number should be printed for invalid message"
        );
        assert!(
            output.contains("BodyLength mismatch"),
            "error annotations should be rendered: {output}"
        );
        assert!(
            output.contains('|'),
            "default display delimiter replacement should appear"
        );
    }

    #[test]
    fn validation_skips_valid_messages() {
        let _lock = TEST_GUARD.lock().unwrap();
        set_validation(true);
        let obfuscator = fix::create_obfuscator(false);
        let lookup = load_dictionary(&format!("8=FIX.4.4{SOH}35=0{SOH}10=000{SOH}"));
        let order = lookup
            .message_def("0")
            .expect("heartbeat definition")
            .field_order
            .clone();
        let mut values = HashMap::new();
        values.insert(35u32, "0");
        values.insert(34u32, "1");
        values.insert(49u32, "AAA");
        values.insert(52u32, "20240101-00:00:00");
        values.insert(56u32, "BBB");

        let body = build_body_from_order(&order, &values);
        let msg_without_checksum = format!("8=FIX.4.4{SOH}9={:03}{SOH}{}", body.len(), body);
        let checksum = validator::calculate_checksum(&format!("{msg_without_checksum}10=000{SOH}"));
        let msg = format!("{msg_without_checksum}10={checksum:03}{SOH}");
        let dict = load_dictionary(&msg);
        let errs = validator::validate_fix_message(&msg, &dict);
        assert!(
            errs.is_clean(),
            "message used for validation bypass should be valid, got {:?}",
            errs.errors
        );
        let line = format!("{msg}\n");
        let mut out = Vec::new();
        stream_reader(
            BufReader::new(Cursor::new(line)),
            &mut out,
            &obfuscator,
            '|',
        )
        .unwrap();
        set_validation(false);

        let output = String::from_utf8(out).unwrap();
        assert!(
            output.trim().is_empty(),
            "valid messages should not produce output in validation mode"
        );
    }

    #[test]
    fn validation_inserts_missing_tags() {
        let _lock = TEST_GUARD.lock().unwrap();
        disable_output_colours();
        set_validation(true);
        let obfuscator = fix::create_obfuscator(false);
        let msg = format!("8=FIX.4.4{SOH}9=005{SOH}10=999{SOH}");
        let line = format!("{msg}\n");
        let mut out = Vec::new();
        stream_reader(
            BufReader::new(Cursor::new(line)),
            &mut out,
            &obfuscator,
            '|',
        )
        .unwrap();
        set_validation(false);

        let output = String::from_utf8(out).unwrap();
        assert!(
            output.contains("35 (MsgType): Missing"),
            "missing tag should be shown in decoded output: {output}"
        );
    }

    #[test]
    fn prettify_includes_missing_tag_annotations_once() {
        let _lock = TEST_GUARD.lock().unwrap();
        disable_output_colours();
        let msg = format!("8=FIX.4.4{SOH}9=005{SOH}35=0{SOH}10=000{SOH}");
        let dict = load_dictionary(&msg);

        let mut report = validator::ValidationReport::default();
        report
            .tag_errors
            .insert(34, vec!["missing sequence".to_string()]);

        let pretty = prettify_with_report(&msg, &dict, Some(&report));
        let lines: Vec<&str> = pretty.lines().collect();
        let missing_lines: Vec<&str> = lines
            .iter()
            .copied()
            .filter(|l| l.contains("34") && l.contains("missing sequence"))
            .collect();

        assert_eq!(
            missing_lines.len(),
            1,
            "missing tag 34 should appear exactly once: {pretty}"
        );
    }

    #[test]
    fn prettify_orders_without_msg_type_header_first() {
        let _lock = TEST_GUARD.lock().unwrap();
        disable_output_colours();
        let msg = format!("8=FIX.4.4{SOH}9=005{SOH}55=IBM{SOH}10=999{SOH}");
        let dict = load_dictionary(&msg);

        let pretty = prettify_with_report(&msg, &dict, None);
        let tags: Vec<u32> = pretty
            .lines()
            .filter_map(|line| line.split_whitespace().next())
            .filter_map(|tag| tag.parse::<u32>().ok())
            .collect();

        assert!(
            tags.starts_with(&[8, 9]),
            "header tags should lead when MsgType is missing: {:?}",
            tags
        );
        let pos_55 = tags.iter().position(|t| *t == 55);
        let pos_10 = tags.iter().position(|t| *t == 10);
        assert!(
            pos_55 < pos_10,
            "body tag 55 should appear before checksum: {:?}",
            tags
        );
    }

    fn build_body_from_order(order: &[u32], values: &HashMap<u32, &str>) -> String {
        let mut out = String::new();
        for tag in order {
            if *tag == 8 || *tag == 9 || *tag == 10 {
                continue;
            }
            if let Some(val) = values.get(tag) {
                out.push_str(&format!("{tag}={val}{SOH}"));
            }
        }
        out
    }
}
