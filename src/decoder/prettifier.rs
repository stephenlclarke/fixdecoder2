// SPDX-License-Identifier: AGPL-3.0-only
// SPDX-FileCopyrightText: 2025 Steve Clarke <stephenlclarke@mac.com> - https://xyzzy.tools

use crate::decoder::colours::{disable_colours, palette};
use crate::decoder::fixparser::parse_fix;
use crate::decoder::tag_lookup::{FixTagLookup, load_dictionary};
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

pub fn validation_enabled() -> bool {
    VALIDATION_ENABLED.load(Ordering::Relaxed)
}

/// Render a single FIX message into a human-friendly string using the provided dictionary.
pub fn prettify(msg: &str, dict: &FixTagLookup) -> String {
    let colours = palette();
    let mut output = String::new();

    for field in parse_fix(msg) {
        let name = dict.field_name(field.tag);
        let desc = dict.enum_description(field.tag, &field.value);
        output.push_str(&format!(
            "    {}{:4}{} ({}{}{}): {}{}{}",
            colours.tag,
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

        output.push('\n');
    }

    output
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
        let errors = validator::validate_fix_message(msg, &dict);
        if errors.is_empty() {
            continue;
        }
        let pretty = prettify(msg, &dict);
        invalid.push((msg.to_string(), pretty, errors));
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

    for (_, pretty, errors) in invalid {
        write!(out, "{pretty}")?;
        for err in errors {
            writeln!(out, "  !! {}", err)?;
        }
        writeln!(out)?;
    }

    Ok(())
}

fn find_fix_message_indices(line: &str) -> Vec<(usize, usize)> {
    FIX_REGEX
        .find_iter(line)
        .map(|m| (m.start(), m.end()))
        .collect()
}

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

fn process_fix_message(msg: &str, out: &mut dyn Write, separator: &str) -> io::Result<()> {
    let dict = load_dictionary(msg);
    let pretty = prettify(msg, &dict);
    write!(out, "{pretty}")?;

    if VALIDATION_ENABLED.load(Ordering::Relaxed) {
        let errors = validator::validate_fix_message(msg, &dict);
        if !errors.is_empty() {
            let colours = palette();
            write!(out, "{separator}")?;
            for err in errors {
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

    const SOH: char = '\u{0001}';

    #[test]
    fn validation_only_outputs_invalid_messages() {
        set_validation(true);
        let obfuscator = fix::create_obfuscator(false);
        let body = format!("35=0{SOH}34=1{SOH}49=AAA{SOH}52=20240101-00:00:00{SOH}56=BBB{SOH}");
        let declared_len = body.as_bytes().len() + 1; // intentionally wrong
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
        let msg_without_checksum =
            format!("8=FIX.4.4{SOH}9={:03}{SOH}{}", body.as_bytes().len(), body);
        let checksum = validator::calculate_checksum(&format!("{msg_without_checksum}10=000{SOH}"));
        let msg = format!("{msg_without_checksum}10={checksum:03}{SOH}");
        let dict = load_dictionary(&msg);
        let errs = validator::validate_fix_message(&msg, &dict);
        assert!(
            errs.is_empty(),
            "message used for validation bypass should be valid, got {:?}",
            errs
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
