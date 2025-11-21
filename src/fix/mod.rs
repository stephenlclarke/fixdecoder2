// SPDX-License-Identifier: AGPL-3.0-only
// SPDX-FileCopyrightText: 2025 Steve Clarke <stephenlclarke@mac.com> - https://xyzzy.tools

pub mod obfuscator;
pub mod sensitive;

mod dictionaries;

pub use obfuscator::Obfuscator;
pub use sensitive::SENSITIVE_TAG_NAMES;

pub fn choose_embedded_xml(version: &str) -> &'static str {
    dictionaries::choose_embedded_xml(version)
}

#[allow(dead_code)]
pub fn supported_fix_versions() -> &'static str {
    "40,41,42,43,44,50,50SP1,50SP2,T11"
}

pub fn create_obfuscator(enabled: bool) -> Obfuscator {
    Obfuscator::from_sensitive_tags(&SENSITIVE_TAG_NAMES, enabled)
}
