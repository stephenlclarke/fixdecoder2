// SPDX-License-Identifier: AGPL-3.0-only
// SPDX-FileCopyrightText: 2025 Steve Clarke <stephenlclarke@mac.com> - https://xyzzy.tools

const FIX40_XML: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/resources/FIX40.xml"));
const FIX41_XML: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/resources/FIX41.xml"));
const FIX42_XML: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/resources/FIX42.xml"));
const FIX43_XML: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/resources/FIX43.xml"));
const FIX44_XML: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/resources/FIX44.xml"));
const FIX50_XML: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/resources/FIX50.xml"));
const FIX50SP1_XML: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/resources/FIX50SP1.xml"
));
const FIX50SP2_XML: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/resources/FIX50SP2.xml"
));
const FIXT11_XML: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/resources/FIXT11.xml"));

pub fn choose_embedded_xml(version: &str) -> &'static str {
    match version.to_ascii_uppercase().as_str() {
        "40" => FIX40_XML,
        "41" => FIX41_XML,
        "42" => FIX42_XML,
        "43" => FIX43_XML,
        "44" => FIX44_XML,
        "50" => FIX50_XML,
        "50SP1" => FIX50SP1_XML,
        "50SP2" => FIX50SP2_XML,
        "T11" | "FIXT11" => FIXT11_XML,
        _ => FIX44_XML,
    }
}
