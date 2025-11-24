// SPDX-License-Identifier: AGPL-3.0-only
// SPDX-FileCopyrightText: 2025 Steve Clarke <stephenlclarke@mac.com> - https://xyzzy.tools

/// fixdecoder command-line entry point and CLI orchestration.
///
/// The binary ties together the dictionary tooling and the streaming FIX log
/// prettifier.  This file is intentionally light on protocol logic; it wires
/// user input into the focused modules under `src/decoder` and `src/fix`.
/// The comments favour UK English and aim to give future maintainers a quick
/// reminder of why each function exists and how it cooperates with the rest
/// of the app.
mod decoder;
mod fix;

use anyhow::{Context, Result, anyhow};
use atty::Stream;
use clap::error::ErrorKind;
use clap::parser::ValueSource;
use clap::{Arg, ArgAction, ArgMatches, Command};
use decoder::{
    DisplayStyle, FixDictionary, disable_output_colours, display_component, display_message,
    list_all_components, list_all_messages, list_all_tags, prettify_files, print_component_columns,
    print_message_columns, print_tag_details, print_tags_in_columns, register_fix_dictionary,
    schema::SchemaTree, set_validation,
};
use std::collections::HashMap;
use std::fs;
use std::io;
use std::sync::OnceLock;

/// Wrapper for a custom FIX dictionary sourced from `--xml` along with its path.
struct CustomDictionary {
    dict: FixDictionary,
    path: String,
}

/// Build-time version information.  The CI pipeline bakes in the most recent
/// tag via `FIXDECODER_VERSION`; otherwise we fall back to Cargo’s package
/// version which tracks the published crate.
const VERSION: &str = match option_env!("FIXDECODER_VERSION") {
    Some(tag) => tag,
    None => env!("CARGO_PKG_VERSION"),
};

/// Determine the current Git branch, defaulting to `main` when the metadata
/// was not injected during the build.  This is UK spelling friendly as the
/// output lands in user-facing banners.
fn branch() -> &'static str {
    option_env!("FIXDECODER_BRANCH").unwrap_or("main")
}

/// Determine the short Git commit that went into the binary.  We rely on CI
/// to provide this, but fall back to a recognisable placeholder.
fn sha() -> &'static str {
    option_env!("FIXDECODER_COMMIT").unwrap_or("0000000")
}

/// Determine the Git remote that best describes the source tree.  Useful
/// when users report bugs and need to know where the code originated.
fn git_url() -> &'static str {
    option_env!("FIXDECODER_GIT_URL").unwrap_or("https://github.com/stephenlclarke/fixdecoder2.git")
}

/// Determine the rustc version baked in at build time.
fn rust_version() -> &'static str {
    option_env!("RUSTC_VERSION").unwrap_or("unknown")
}

/// Human-friendly version banner including branch and commit.
fn version_string() -> String {
    format!(
        "fixdecoder {VERSION} (branch:{}, commit:{}) [rust:{}]",
        branch(),
        sha(),
        rust_version()
    )
}

/// Cached version string with a 'static lifetime for clap metadata.
fn version_str() -> &'static str {
    static VERSION_STR: OnceLock<String> = OnceLock::new();
    VERSION_STR.get_or_init(version_string).as_str()
}

/// Conventional `main` that defers to `run` so tests can call the logic
/// without having to spin up a separate process.
fn main() {
    std::process::exit(match run() {
        Ok(code) => code,
        Err(err) => {
            eprintln!("{err}");
            1
        }
    });
}

/// Parse CLI arguments, load dictionaries, respond to informational flags
/// and finally drive the prettifier.  Everything user-facing goes through
/// here, so the structure favours clarity over cleverness.
fn run() -> Result<i32> {
    println!("{}", version_string());

    let cmd = build_cli();
    let matches = match cmd.try_get_matches() {
        Ok(m) => m,
        Err(err) => match err.kind() {
            ErrorKind::DisplayHelp | ErrorKind::DisplayVersion => {
                err.print()?;
                if err.kind() == ErrorKind::DisplayHelp {
                    print_usage();
                }
                return Ok(0);
            }
            _ => err.exit(),
        },
    };

    let opts = CliOptions::from_matches(&matches)?;

    if opts.show_version {
        print_git_clone();
        return Ok(0);
    }

    set_validation(opts.validate);

    let custom_dicts = load_custom_dictionaries(&opts.xml_paths)?;
    ensure_valid_fix_version(&opts, &custom_dicts)?;
    let schema = load_schema(&opts, &custom_dicts)?;

    if run_handlers(&opts, &schema, &custom_dicts)? {
        return Ok(0);
    }

    if let Some(force_colour) = opts.colour {
        if !force_colour {
            disable_output_colours();
        }
    } else if !atty::is(Stream::Stdout) {
        disable_output_colours();
    }

    let obfuscator = fix::create_obfuscator(opts.secret);
    let files = if opts.files.is_empty() {
        vec!["-".to_string()]
    } else {
        opts.files.clone()
    };

    let mut stdout = io::stdout();
    let mut stderr = io::stderr();
    let code = prettify_files(
        &files,
        &mut stdout,
        &mut stderr,
        &obfuscator,
        opts.delimiter,
    );
    Ok(code)
}

/// Construct the `clap` command with all supported arguments.  Options are
/// grouped roughly by feature area (dictionary browsing, validation, IO).
fn build_cli() -> Command {
    let mut cmd = Command::new("fixdecoder")
        .about("FIX protocol utility - Dictionary lookup, file decoder, validator & prettifier")
        .disable_version_flag(true)
        .version(version_str())
        .arg(
            Arg::new("fix")
                .long("fix")
                .value_name("VER")
                .default_value("44")
                .help("FIX version to use"),
        )
        .arg(
            Arg::new("xml")
                .long("xml")
                .value_name("FILE")
                .action(ArgAction::Append)
                .help("Path to alternative FIX XML dictionary (repeatable)"),
        );

    cmd = add_entity_arg(
        cmd,
        "message",
        "MSG",
        "FIX Message name or MsgType (omit value to list all)",
    );
    cmd = add_entity_arg(
        cmd,
        "component",
        "NAME",
        "FIX Component to display (omit value to list all)",
    );
    cmd = add_entity_arg(
        cmd,
        "tag",
        "TAG",
        "FIX Tag number to display (omit value to list all)",
    );

    cmd = add_flag_args(
        cmd,
        &[
            ("column", "Display enums in columns"),
            ("header", "Include Header block"),
            ("trailer", "Include Trailer block"),
            ("verbose", "Show full message structure with enums"),
            ("info", "Show schema summary"),
            ("secret", "Obfuscate sensitive FIX tag values"),
            ("validate", "Validate FIX messages during decoding"),
        ],
    );

    cmd.arg(
        Arg::new("colour")
            .long("colour")
            .num_args(0..=1)
            .value_name("yes|no")
            .require_equals(false)
            .default_missing_value("true")
            .help("Force coloured output"),
    )
    .arg(
        Arg::new("delimiter")
            .long("delimiter")
            .value_name("CHAR")
            .help("Display delimiter between FIX fields (default: SOH)"),
    )
    .arg(
        Arg::new("version")
            .long("version")
            .action(ArgAction::SetTrue)
            .help("Print version information and exit"),
    )
    .arg(
        Arg::new("files")
            .value_name("FILE")
            .num_args(0..)
            .action(ArgAction::Append)
            .trailing_var_arg(true),
    )
}

/// Add a `--name[=VALUE]` argument that can be used with or without a value (defaulting to “true”).
fn add_entity_arg(
    cmd: Command,
    name: &'static str,
    value_name: &'static str,
    help: &'static str,
) -> Command {
    cmd.arg(
        Arg::new(name)
            .long(name)
            .num_args(0..=1)
            .value_name(value_name)
            .require_equals(false)
            .default_missing_value("true")
            .help(help),
    )
}

/// Add a set of boolean flag arguments that simply flip a boolean when present.
fn add_flag_args(cmd: Command, flags: &[(&'static str, &'static str)]) -> Command {
    let mut out = cmd;
    for (name, help) in flags {
        out = out.arg(
            Arg::new(*name)
                .long(*name)
                .action(ArgAction::SetTrue)
                .help(*help),
        );
    }
    out
}

/// Structured view of the CLI flags so downstream code gets type-safe access
/// to user intent.
struct CliOptions {
    fix_version: String,
    fix_from_user: bool,
    xml_paths: Vec<String>,
    message_flag: bool,
    message_value: Option<String>,
    component_flag: bool,
    component_value: Option<String>,
    tag_flag: bool,
    tag_value: Option<String>,
    column: bool,
    verbose: bool,
    include_header: bool,
    include_trailer: bool,
    info: bool,
    secret: bool,
    validate: bool,
    colour: Option<bool>,
    show_version: bool,
    files: Vec<String>,
    delimiter: char,
}

impl CliOptions {
    /// Translate clap’s `ArgMatches` into our strongly typed `CliOptions`.
    /// The function centralises validation so the rest of the code can assume
    /// sane defaults and bail out early when a user supplies nonsense.
    fn from_matches(matches: &ArgMatches) -> Result<Self> {
        let fix_source = matches.value_source("fix");
        let fix_from_user = fix_source != Some(ValueSource::DefaultValue);

        let xml_paths: Vec<String> = matches
            .get_many::<String>("xml")
            .map(|vals| vals.map(|v| v.to_string()).collect())
            .unwrap_or_default();

        let files: Vec<String> = matches
            .get_many::<String>("files")
            .map(|vals| vals.map(|v| v.to_string()).collect())
            .unwrap_or_default();
        Ok(Self {
            fix_version: matches
                .get_one::<String>("fix")
                .cloned()
                .unwrap_or_else(|| "44".to_string()),
            fix_from_user,
            xml_paths,
            message_flag: matches.contains_id("message"),
            message_value: extract_optional_arg(matches, "message")?,
            component_flag: matches.contains_id("component"),
            component_value: extract_optional_arg(matches, "component")?,
            tag_flag: matches.contains_id("tag"),
            tag_value: extract_optional_arg(matches, "tag")?,
            column: matches.get_flag("column"),
            verbose: matches.get_flag("verbose"),
            include_header: matches.get_flag("header"),
            include_trailer: matches.get_flag("trailer"),
            info: matches.get_flag("info"),
            secret: matches.get_flag("secret"),
            validate: matches.get_flag("validate"),
            colour: parse_colour(matches.get_one::<String>("colour"))?,
            show_version: matches.get_flag("version"),
            files,
            delimiter: parse_delimiter(matches.get_one::<String>("delimiter"))?,
        })
    }
}

/// Handle flags that may be specified with or without a value (such as
/// `--message` or `--tag`).  We treat an empty string as a user error and
/// show the usage banner straight away.
fn extract_optional_arg(matches: &ArgMatches, name: &str) -> Result<Option<String>> {
    if let Some(value) = matches.get_one::<String>(name) {
        if value.is_empty() {
            print_usage();
            return Err(anyhow!("Invalid value for --{name}"));
        }
        if value == "true" {
            return Ok(None);
        }
        return Ok(Some(value.clone()));
    }
    Ok(None)
}

/// Interpret command-line colour overrides, keeping support for human-friendly
/// words like “yes” and “no”.  This is kept separate so unit tests can focus
/// on the parsing logic.
fn parse_colour(value: Option<&String>) -> Result<Option<bool>> {
    match value {
        None => Ok(None),
        Some(v) if v.is_empty() => Ok(None),
        Some(v) => match v.to_ascii_lowercase().as_str() {
            "true" | "yes" => Ok(Some(true)),
            "false" | "no" => Ok(Some(false)),
            other => {
                print_usage();
                Err(anyhow!("invalid value for --colour: {other}"))
            }
        },
    }
}

/// Load all custom dictionary files specified via `--xml`, registering them and
/// returning the key-to-dictionary map. Emits warnings on overrides.
fn load_custom_dictionaries(paths: &[String]) -> Result<HashMap<String, CustomDictionary>> {
    let mut dicts = HashMap::new();
    let builtin_keys = built_in_fix_keys();
    for path in paths {
        let xml_data =
            fs::read_to_string(path).with_context(|| format!("failed to read {path}"))?;
        let mut dict = FixDictionary::from_xml(&xml_data)
            .with_context(|| format!("failed to parse FIX XML from {path}"))?;
        let key = dictionary_key(&dict);
        ensure_session_components(&key, &mut dict);
        register_fix_dictionary(&key, &dict);
        if let Some(existing) = dicts.insert(
            key.clone(),
            CustomDictionary {
                dict,
                path: path.to_string(),
            },
        ) {
            eprintln!(
                "warning: custom dictionary for {key} from {} replaced by {}\n",
                existing.path, path
            );
        } else if builtin_keys.contains(&key) {
            eprintln!(
                "warning: custom dictionary for {key} overrides embedded dictionary using {}\n",
                path
            );
        }
    }
    Ok(dicts)
}

/// Load an embedded FIX dictionary by version string (e.g. "44").
fn load_embedded_dictionary(fix_version: &str) -> Result<FixDictionary> {
    let xml_data = fix::choose_embedded_xml(fix_version);
    FixDictionary::from_xml(xml_data).with_context(|| "failed to parse FIX XML")
}

/// Load an embedded FIX dictionary by canonical key (e.g. "FIX44").
fn load_embedded_dictionary_for_key(key: &str) -> Result<FixDictionary> {
    let xml_id = key_to_xml_id(key).ok_or_else(|| anyhow!("no embedded dictionary for {key}"))?;
    let xml_data = fix::choose_embedded_xml(xml_id);
    FixDictionary::from_xml(xml_data)
        .with_context(|| format!("failed to parse embedded FIX XML for {key}"))
}

/// Parse the delimiter override supplied on the CLI.  Users can pass a
/// literal character, “SOH”, or a hex escape like `\x1f`.  The parser errs
/// on the side of helpful messages whilst staying strict.
fn parse_delimiter(value: Option<&String>) -> Result<char> {
    const SOH: char = '\u{0001}';
    match value {
        None => Ok(SOH),
        Some(v) if v.is_empty() => Err(anyhow!("delimiter cannot be empty")),
        Some(v) => {
            if v.eq_ignore_ascii_case("SOH") {
                return Ok(SOH);
            }
            if let Some(hex) = v.strip_prefix("\\x").or_else(|| v.strip_prefix("0x")) {
                let code = u32::from_str_radix(hex, 16)
                    .map_err(|_| anyhow!("invalid delimiter hex value: {v}"))?;
                return char::from_u32(code)
                    .ok_or_else(|| anyhow!("delimiter code {v} is not valid Unicode"));
            }
            if v.chars().count() == 1 {
                return Ok(v.chars().next().unwrap());
            }
            Err(anyhow!(
                "delimiter must be a single character or hex code like \\x01"
            ))
        }
    }
}

/// Load the requested FIX dictionary for CLI queries.  Custom dictionaries
/// loaded via `--xml` are preferred when they match the requested FIX version,
/// otherwise the embedded defaults are used.  FIXT11 session components are
/// merged when a FIX 5.0+ application dictionary omits them.
fn load_schema(
    opts: &CliOptions,
    custom_dicts: &HashMap<String, CustomDictionary>,
) -> Result<SchemaTree> {
    let normalized_key = normalise_fix_key(&opts.fix_version);

    let mut dict = if let Some(ref key) = normalized_key
        && let Some(custom) = custom_dicts.get(key)
    {
        custom.dict.clone()
    } else {
        load_embedded_dictionary(&opts.fix_version)?
    };

    let dict_key = dictionary_key(&dict);
    ensure_session_components(&dict_key, &mut dict);

    Ok(SchemaTree::build(dict))
}

/// Load a dictionary for a specific canonical key, preferring custom entries when present.
fn load_schema_for_key(
    key: &str,
    custom_dicts: &HashMap<String, CustomDictionary>,
) -> Result<SchemaTree> {
    let normalized = key.to_ascii_uppercase();
    let mut dict = if let Some(custom) = custom_dicts.get(&normalized) {
        custom.dict.clone()
    } else {
        load_embedded_dictionary_for_key(&normalized)?
    };
    ensure_session_components(&normalized, &mut dict);
    Ok(SchemaTree::build(dict))
}

/// Handle non-streaming commands such as `--message`, `--tag`, `--component`
/// and `--info`.  Returns `true` when an action was performed so the caller
/// can skip the prettifier.
fn run_handlers(
    opts: &CliOptions,
    schema: &SchemaTree,
    custom_dicts: &HashMap<String, CustomDictionary>,
) -> Result<bool> {
    let mut handled = false;

    if opts.info {
        handle_info(opts, schema, custom_dicts)?;
        handled = true;
    }

    if opts.message_flag {
        handle_messages(opts, schema)?;
        handled = true;
    }

    if opts.tag_flag {
        handle_tags(opts, schema)?;
        handled = true;
    }

    if opts.component_flag {
        handle_components(opts, schema)?;
        handled = true;
    }

    Ok(handled)
}

/// Ensure user-supplied FIX versions map to either built-in or custom dictionaries.
fn ensure_valid_fix_version(
    opts: &CliOptions,
    custom_dicts: &HashMap<String, CustomDictionary>,
) -> Result<()> {
    if !opts.fix_from_user {
        return Ok(());
    }

    if let Some(key) = normalise_fix_key(&opts.fix_version) {
        let builtin = built_in_fix_keys();
        if builtin.contains(&key) || custom_dicts.contains_key(&key) {
            return Ok(());
        }
    }

    eprintln!("Invalid --fix value: {}", opts.fix_version);
    print_usage();
    Err(anyhow!("invalid --fix value"))
}

/// Locate a message definition by name or MsgType, returning the matching node if found.
fn find_message<'a>(
    schema: &'a SchemaTree,
    query: &str,
) -> Option<&'a decoder::schema::MessageNode> {
    schema
        .messages
        .get(query)
        .or_else(|| schema.messages.values().find(|m| m.msg_type == query))
}

fn print_git_clone() {
    println!("  git clone {}", git_url());
}
/// Print the condensed usage guide.  Kept in one function so we can reuse it
/// whenever argument parsing fails.
fn print_usage() {
    static USAGE: &str = include_str!("../resources/messages/usage_en.txt");
    println!("\n{USAGE}");
}

/// Normalise user-supplied FIX version identifiers (e.g. `4.4`, `fix44`)
/// into the canonical keys used throughout the project.
fn normalise_fix_key(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }

    let mut cleaned = trimmed.replace('.', "");
    cleaned = cleaned.to_ascii_uppercase();

    if cleaned.starts_with("FIX") {
        Some(cleaned)
    } else {
        Some(format!("FIX{}", cleaned))
    }
}

/// Derive the canonical dictionary key (e.g. FIX40SP1) from a parsed dictionary.
fn dictionary_key(dict: &FixDictionary) -> String {
    let prefix = if dict.typ.eq_ignore_ascii_case("FIXT") {
        "FIXT"
    } else {
        "FIX"
    };

    let mut key = format!("{}{}{}", prefix, dict.major, dict.minor);
    if let Some(sp) = dict
        .service_pack
        .as_deref()
        .filter(|s| !s.is_empty() && s != &"0")
    {
        key.push_str("SP");
        key.push_str(&sp.to_ascii_uppercase());
    }
    key.to_ascii_uppercase()
}

/// Return the set of built-in FIX dictionary keys shipped with the binary.
fn built_in_fix_keys() -> Vec<String> {
    vec![
        "FIX27", "FIX30", "FIX40", "FIX41", "FIX42", "FIX43", "FIX44", "FIX50", "FIX50SP1",
        "FIX50SP2", "FIXT11",
    ]
    .into_iter()
    .map(|s| s.to_string())
    .collect()
}

/// Combine built-in and custom dictionary keys for informational listings.
fn all_dictionary_keys(custom_dicts: &HashMap<String, CustomDictionary>) -> Vec<String> {
    let mut versions = built_in_fix_keys();
    for key in custom_dicts.keys() {
        if !versions.contains(key) {
            versions.push(key.clone());
        }
    }
    versions.sort();
    versions
}

/// Render the available dictionary keys as a comma-separated list.
fn available_fix_versions(custom_dicts: &HashMap<String, CustomDictionary>) -> String {
    all_dictionary_keys(custom_dicts).join(",")
}

/// Return the source path for a dictionary key, falling back to “built-in”.
fn dictionary_source(custom_dicts: &HashMap<String, CustomDictionary>, key: &str) -> String {
    let normalized = key.to_ascii_uppercase();
    custom_dicts
        .get(&normalized)
        .map(|c| c.path.clone())
        .unwrap_or_else(|| "built-in".to_string())
}

/// Print the table header for dictionary listings.
fn print_dictionary_header() {
    println!(
        "  {:<10} {:>12} {:>8} {:>11} {:>11} Source",
        "Version", "ServicePack", "Fields", "Components", "Messages",
    );
}

/// Print one row of dictionary metadata.
fn print_dictionary_row(key: &str, schema: &SchemaTree, source: &str) {
    println!(
        "  {:<10} {:>12} {:>8} {:>11} {:>11} {}",
        key,
        schema.service_pack,
        schema.fields.len(),
        schema.components.len(),
        schema.messages.len(),
        source
    );
}

/// Determine whether a particular FIX dictionary needs the FIXT11 session
/// header/trailer merged in.  Saves the rest of the code from hard-coding
/// these version checks repeatedly.
fn requires_session_components(key: &str) -> bool {
    matches!(key, "FIX50" | "FIX50SP1" | "FIX50SP2")
}

/// Supply header/trailer blocks from FIXT11 into FIX 5.0+ dictionaries when absent.
fn ensure_session_components(key: &str, dict: &mut FixDictionary) {
    if !requires_session_components(key) {
        return;
    }

    let session_xml = fix::choose_embedded_xml("T11");
    let session = match FixDictionary::from_xml(session_xml) {
        Ok(dict) => dict,
        Err(err) => {
            eprintln!("warning: failed to load FIXT11 session dictionary ({err})");
            return;
        }
    };

    if !component_def_has_entries(&dict.header) {
        dict.header = session.header;
    }
    if !component_def_has_entries(&dict.trailer) {
        dict.trailer = session.trailer;
    }
}

fn component_def_has_entries(block: &decoder::schema::ComponentDef) -> bool {
    !block.fields.is_empty() || !block.groups.is_empty() || !block.components.is_empty()
}

/// Map a canonical FIX key to the embedded XML identifier used by `choose_embedded_xml`.
fn key_to_xml_id(key: &str) -> Option<&'static str> {
    match key.to_ascii_uppercase().as_str() {
        "FIX27" => Some("40"),
        "FIX30" => Some("40"),
        "FIX40" => Some("40"),
        "FIX41" => Some("41"),
        "FIX42" => Some("42"),
        "FIX43" => Some("43"),
        "FIX44" => Some("44"),
        "FIX50" => Some("50"),
        "FIX50SP1" => Some("50SP1"),
        "FIX50SP2" => Some("50SP2"),
        "FIXT11" => Some("T11"),
        _ => None,
    }
}

/// Print a summary table of all available dictionaries (built-in and custom).
fn print_all_dictionary_info(custom_dicts: &HashMap<String, CustomDictionary>) -> Result<()> {
    println!(
        "Available FIX Dictionaries: {}",
        available_fix_versions(custom_dicts)
    );
    println!("\nLoaded dictionaries:");
    print_dictionary_header();

    for key in all_dictionary_keys(custom_dicts) {
        match load_schema_for_key(&key, custom_dicts) {
            Ok(schema) => {
                let source = dictionary_source(custom_dicts, &key);
                print_dictionary_row(&key, &schema, &source);
            }
            Err(err) => eprintln!("warning: failed to load {key}: {err}"),
        }
    }
    println!();
    Ok(())
}

/// Handle the `--info` command, printing either all dictionaries or the selected one.
fn handle_info(
    opts: &CliOptions,
    schema: &SchemaTree,
    custom_dicts: &HashMap<String, CustomDictionary>,
) -> Result<()> {
    if opts.fix_from_user {
        println!(
            "Available FIX Dictionaries: {}",
            available_fix_versions(custom_dicts)
        );
        println!("\nCurrent Schema:");
        print_dictionary_header();
        let key = normalise_fix_key(&opts.fix_version).unwrap_or_else(|| "FIX44".to_string());
        let source = dictionary_source(custom_dicts, &key);
        print_dictionary_row(&key, schema, &source);
        println!();
    } else {
        print_all_dictionary_info(custom_dicts)?;
    }
    Ok(())
}

/// Handle `--message` mode (list or render a specific message).
fn handle_messages(opts: &CliOptions, schema: &SchemaTree) -> Result<()> {
    match &opts.message_value {
        None => {
            if opts.column {
                print_message_columns(schema)?;
            } else {
                list_all_messages(schema)?;
            }
        }
        Some(value) => {
            if let Some(message) = find_message(schema, value) {
                let style = DisplayStyle::new(decoder::colours::palette(), opts.column);
                display_message(
                    schema,
                    message,
                    opts.verbose,
                    opts.include_header,
                    opts.include_trailer,
                    4,
                    style,
                )?;
            } else {
                println!("Message not found: {value}");
            }
        }
    }
    Ok(())
}

/// Handle `--tag` mode (list or show details).
fn handle_tags(opts: &CliOptions, schema: &SchemaTree) -> Result<()> {
    match &opts.tag_value {
        None => {
            if opts.column {
                print_tags_in_columns(schema)?;
            } else {
                list_all_tags(schema)?;
            }
        }
        Some(value) => {
            let tag: u32 = value.parse().map_err(|_| anyhow!("Invalid tag: {value}"))?;
            if let Some(field) = schema.find_field_by_number(tag) {
                print_tag_details(field, opts.verbose, opts.column)?;
            } else {
                println!("Tag not found: {tag}");
            }
        }
    }
    Ok(())
}

/// Handle `--component` mode (list or render a specific component).
fn handle_components(opts: &CliOptions, schema: &SchemaTree) -> Result<()> {
    match &opts.component_value {
        None => {
            if opts.column {
                print_component_columns(schema)?;
            } else {
                list_all_components(schema)?;
            }
        }
        Some(name) => {
            if let Some(component) = schema.components.get(name) {
                let style = DisplayStyle::new(decoder::colours::palette(), opts.column);
                display_component(schema, None, component, opts.verbose, 0, style)?;
            } else {
                println!("Component not found: {name}");
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn dummy_opts(version: &str) -> CliOptions {
        CliOptions {
            fix_version: version.to_string(),
            fix_from_user: true,
            xml_paths: Vec::new(),
            message_flag: false,
            message_value: None,
            component_flag: false,
            component_value: None,
            tag_flag: false,
            tag_value: None,
            column: false,
            verbose: false,
            include_header: false,
            include_trailer: false,
            info: false,
            secret: false,
            validate: false,
            colour: None,
            show_version: false,
            files: Vec::new(),
            delimiter: '\u{0001}',
        }
    }

    #[test]
    fn version_string_matches_components() {
        let expected = format!(
            "fixdecoder {VERSION} (branch:{}, commit:{}) [rust:{}]",
            branch(),
            sha(),
            rust_version()
        );
        assert_eq!(version_string(), expected);
    }

    #[test]
    fn version_str_is_cached() {
        let first = version_str() as *const str;
        let second = version_str() as *const str;
        assert_eq!(first, second, "cached version string should be stable");
    }

    #[test]
    fn invalid_fix_version_errors() {
        let opts = dummy_opts("45");
        let res = ensure_valid_fix_version(&opts, &HashMap::new());
        assert!(res.is_err());
    }

    #[test]
    fn valid_fix_version_passes() {
        let opts = dummy_opts("44");
        let res = ensure_valid_fix_version(&opts, &HashMap::new());
        assert!(res.is_ok());
    }
}
