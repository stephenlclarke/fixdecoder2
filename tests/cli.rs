// SPDX-License-Identifier: AGPL-3.0-only
// Integration smoke tests for the CLI to ensure end-to-end flows keep working.

use assert_cmd::cargo::cargo_bin_cmd;
use predicates::prelude::PredicateBooleanExt;
use predicates::str::contains;
use std::io::Write;
use tempfile::NamedTempFile;

fn fix_message(body: &str) -> String {
    let soh = '\u{0001}';
    format!("8=FIX.4.4{soh}9=005{soh}{body}10=000{soh}\n")
}

#[test]
fn decodes_single_message_from_stdin() {
    let msg = fix_message("35=0");
    cargo_bin_cmd!("fixdecoder")
        .arg("--fix=44")
        .write_stdin(msg)
        .assert()
        .success()
        .stdout(contains("BeginString").and(contains("MsgType")));
}

#[test]
fn validation_reports_missing_fields() {
    let msg = fix_message(""); // missing MsgType intentionally
    cargo_bin_cmd!("fixdecoder")
        .args(["--fix=44", "--validate"])
        .write_stdin(msg)
        .assert()
        .success()
        .stdout(contains("Line 1:").and(contains("MsgType").and(contains("Missing"))));
}

#[test]
fn decodes_message_from_file_path() {
    let mut file = NamedTempFile::new().expect("temp file");
    let msg = fix_message("35=0");
    write!(file, "{msg}").expect("write temp");
    cargo_bin_cmd!("fixdecoder")
        .args(["--fix=44"])
        .arg(file.path())
        .assert()
        .success()
        .stdout(contains("BeginString"));
}

#[test]
fn summary_mode_outputs_order_summary() {
    let mut file = NamedTempFile::new().expect("temp file");
    let soh = '\u{0001}';
    let msg1 = format!("8=FIX.4.4{soh}9=005{soh}35=8{soh}37=O1{soh}11=C1{soh}10=000{soh}\n");
    let msg2 = format!("8=FIX.4.4{soh}9=005{soh}35=8{soh}37=O1{soh}11=C1{soh}10=000{soh}\n");
    write!(file, "{msg1}{msg2}").expect("write temp");
    cargo_bin_cmd!("fixdecoder")
        .args(["--fix=44", "--summary"])
        .arg(file.path())
        .assert()
        .success()
        .stdout(
            contains("Order Summary").and(contains("Execution Report").or(contains("EXECUTION"))),
        );
}

#[test]
fn override_is_honoured_with_fallback() {
    let soh = '\u{0001}';
    let msg = format!("8=FIXT.1.1{soh}9=005{soh}35=0{soh}1128=8{soh}10=000{soh}\n");
    cargo_bin_cmd!("fixdecoder")
        .args(["--fix=44"])
        .write_stdin(msg)
        .assert()
        .success()
        .stdout(contains("ApplVerID"));
}
