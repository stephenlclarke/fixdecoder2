// SPDX-License-Identifier: AGPL-3.0-only
// SPDX-FileCopyrightText: 2025 Steve Clarke <stephenlclarke@mac.com> - https://xyzzy.tools

use std::sync::atomic::{AtomicBool, Ordering};

/// ANSI colour palette used across decoder output. The fields hold the SGR sequences for each role.
#[derive(Clone, Copy)]
pub struct ColourPalette {
    pub reset: &'static str,
    pub line: &'static str,
    pub tag: &'static str,
    pub name: &'static str,
    pub value: &'static str,
    pub enumeration: &'static str,
    pub file: &'static str,
    pub error: &'static str,
    pub message: &'static str,
    pub title: &'static str,
}

const COLOURED: ColourPalette = ColourPalette {
    reset: "\u{001b}[0m",
    line: "\u{001b}[38;5;244m",
    tag: "\u{001b}[38;5;81m",
    name: "\u{001b}[38;5;151m",
    value: "\u{001b}[38;5;228m",
    enumeration: "\u{001b}[38;5;214m",
    file: "\u{001b}[95m",
    error: "\u{001b}[31m",
    message: "\u{001b}[97m",
    title: "\u{001b}[31m",
};

const PLAIN: ColourPalette = ColourPalette {
    reset: "",
    line: "",
    tag: "",
    name: "",
    value: "",
    enumeration: "",
    file: "",
    error: "",
    message: "",
    title: "",
};

static ENABLED: AtomicBool = AtomicBool::new(true);

/// Return the current colour palette, respecting the global enable/disable flag.
pub fn palette() -> ColourPalette {
    if ENABLED.load(Ordering::Relaxed) {
        COLOURED
    } else {
        PLAIN
    }
}

/// Disable ANSI colour output globally (used when piping or when explicitly requested).
pub fn disable_colours() {
    ENABLED.store(false, Ordering::Relaxed);
}
