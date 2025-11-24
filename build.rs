// SPDX-License-Identifier: AGPL-3.0-only
// SPDX-FileCopyrightText: 2025 Steve Clarke <stephenlclarke@mac.com> - https://xyzzy.tools

use std::process::Command;

// Capture build metadata (rustc version, git commit) at build time so the binary
// can report it in --version even outside CI.
fn main() {
    let rustc = rustc_version::version()
        .map(|v| v.to_string())
        .unwrap_or_else(|_| "unknown".to_string());
    println!("cargo:rustc-env=RUSTC_VERSION={rustc}");

    let commit = std::env::var("FIXDECODER_COMMIT")
        .ok()
        .filter(|v| !v.is_empty())
        .or_else(|| git_output(&["rev-parse", "--short", "HEAD"]))
        .unwrap_or_else(|| "0000000".to_string());
    println!("cargo:rustc-env=FIXDECODER_COMMIT={commit}");
}

fn git_output(args: &[&str]) -> Option<String> {
    Command::new("git")
        .args(args)
        .output()
        .ok()
        .and_then(|out| {
            if out.status.success() {
                let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
                if s.is_empty() { None } else { Some(s) }
            } else {
                None
            }
        })
}
