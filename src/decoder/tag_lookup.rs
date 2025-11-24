// SPDX-License-Identifier: AGPL-3.0-only
// SPDX-FileCopyrightText: 2025 Steve Clarke <stephenlclarke@mac.com> - https://xyzzy.tools

use crate::decoder::schema::{ComponentDef, FixDictionary, GroupDef, Message, MessageContainer};
use crate::fix;
use once_cell::sync::Lazy;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, RwLock};

#[derive(Clone, Debug)]
pub struct MessageDef {
    pub _name: String,
    pub _msg_type: String,
    pub field_order: Vec<u32>,
    pub required: Vec<u32>,
}

#[derive(Debug, Default)]
pub struct FixTagLookup {
    tag_to_name: HashMap<u32, String>,
    enum_map: HashMap<u32, HashMap<String, String>>,
    field_types: HashMap<u32, String>,
    messages: HashMap<String, MessageDef>,
    repeatable_tags: HashSet<u32>,
}

impl FixTagLookup {
    pub fn from_dictionary(dict: &FixDictionary) -> Self {
        let mut tag_to_name = HashMap::new();
        let mut enum_map = HashMap::new();
        let mut field_types = HashMap::new();
        let mut name_to_tag = HashMap::new();
        let mut component_map: HashMap<String, ComponentDef> = HashMap::new();

        for field in &dict.fields.items {
            tag_to_name.insert(field.number, field.name.clone());
            name_to_tag.insert(field.name.clone(), field.number);
            field_types.insert(field.number, field.field_type.clone());

            let mut enums = HashMap::new();
            for value in field.values_iter() {
                enums.insert(value.enumeration.clone(), value.description.clone());
            }
            if !enums.is_empty() {
                enum_map.insert(field.number, enums);
            }
        }

        for comp in dict.components.items.iter() {
            component_map.insert(comp.name.clone(), comp.clone());
        }
        let mut header = dict.header.clone();
        header.name = "Header".to_string();
        component_map.insert(header.name.clone(), header);
        let mut trailer = dict.trailer.clone();
        trailer.name = "Trailer".to_string();
        component_map.insert(trailer.name.clone(), trailer);

        let messages = build_message_defs(&dict.messages, &component_map, &name_to_tag);
        let repeatable_tags = collect_repeatable_tags(&dict.messages, &component_map, &name_to_tag);

        FixTagLookup {
            tag_to_name,
            enum_map,
            field_types,
            messages,
            repeatable_tags,
        }
    }

    pub fn field_name(&self, tag: u32) -> String {
        self.tag_to_name
            .get(&tag)
            .cloned()
            .unwrap_or_else(|| tag.to_string())
    }

    pub fn enum_description(&self, tag: u32, value: &str) -> Option<&str> {
        self.enum_map
            .get(&tag)
            .and_then(|enums| enums.get(value).map(|s| s.as_str()))
    }

    pub fn enums_for(&self, tag: u32) -> Option<&HashMap<String, String>> {
        self.enum_map.get(&tag)
    }

    pub fn field_type(&self, tag: u32) -> Option<&str> {
        self.field_types.get(&tag).map(|s| s.as_str())
    }

    pub fn message_def(&self, msg_type: &str) -> Option<&MessageDef> {
        self.messages.get(msg_type)
    }

    pub fn is_repeatable(&self, tag: u32) -> bool {
        self.repeatable_tags.contains(&tag)
    }
}

static LOOKUPS: Lazy<RwLock<HashMap<String, Arc<FixTagLookup>>>> =
    Lazy::new(|| RwLock::new(HashMap::new()));

const SESSION_KEY: &str = "FIXT11";

fn schema_to_xml_id(key: &str) -> Option<&'static str> {
    match key {
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

fn needs_session_merge(key: &str) -> bool {
    matches!(key, "FIX50" | "FIX50SP1" | "FIX50SP2")
}

fn merge_lookup(dst: &mut FixTagLookup, src: &FixTagLookup) {
    for (tag, name) in &src.tag_to_name {
        dst.tag_to_name.entry(*tag).or_insert_with(|| name.clone());
    }

    for (tag, enums) in &src.enum_map {
        let entry = dst.enum_map.entry(*tag).or_default();
        for (value, desc) in enums {
            entry.entry(value.clone()).or_insert_with(|| desc.clone());
        }
    }

    for (tag, typ) in &src.field_types {
        dst.field_types.entry(*tag).or_insert_with(|| typ.clone());
    }
}

fn get_dictionary(key: &str) -> Option<Arc<FixTagLookup>> {
    if let Some(existing) = LOOKUPS.read().ok()?.get(key).cloned() {
        return Some(existing);
    }

    let xml_id = schema_to_xml_id(key)?;
    let xml = fix::choose_embedded_xml(xml_id);
    let dict = match FixDictionary::from_xml(xml) {
        Ok(dict) => dict,
        Err(err) => {
            eprintln!("failed to parse embedded FIX XML for {key}: {err}");
            return None;
        }
    };
    let lookup = build_lookup_from_dict(key, &dict);

    let arc = Arc::new(lookup);
    let mut guard = LOOKUPS.write().ok()?;
    let entry = guard.entry(key.to_string()).or_insert_with(|| arc.clone());
    Some(entry.clone())
}

fn get_tag_value<'a>(msg: &'a str, tag: &str) -> Option<&'a str> {
    for field in msg.split('\u{0001}') {
        if let Some((lhs, rhs)) = field.split_once('=')
            && lhs == tag
        {
            return Some(rhs);
        }
    }
    None
}

fn detect_schema_key(msg: &str) -> String {
    if let Some(begin) = get_tag_value(msg, "8") {
        if begin == "FIXT.1.1" {
            if let Some(appl_ver_id) =
                get_tag_value(msg, "1128").or_else(|| get_tag_value(msg, "1137"))
                && let Some(schema) = appl_ver_to_schema(appl_ver_id)
            {
                return schema.to_string();
            }
            return "FIX50".to_string();
        }
        return begin.replace('.', "");
    }
    "FIX44".to_string()
}

fn appl_ver_to_schema(value: &str) -> Option<&'static str> {
    match value {
        "0" => Some("FIX27"),
        "1" => Some("FIX30"),
        "2" => Some("FIX40"),
        "3" => Some("FIX41"),
        "4" => Some("FIX42"),
        "5" => Some("FIX43"),
        "6" => Some("FIX44"),
        "7" => Some("FIX50"),
        "8" => Some("FIX50SP1"),
        "9" => Some("FIX50SP2"),
        _ => None,
    }
}

pub fn load_dictionary(msg: &str) -> Arc<FixTagLookup> {
    let key = detect_schema_key(msg);
    get_dictionary(&key)
        .or_else(|| get_dictionary("FIX44"))
        .expect("FIX44 dictionary available")
}

pub fn register_dictionary(key: &str, dict: &FixDictionary) {
    let lookup = build_lookup_from_dict(key, dict);
    let mut guard = LOOKUPS.write().expect("dictionary cache poisoned");
    guard.insert(key.to_string(), Arc::new(lookup));
}

fn build_lookup_from_dict(key: &str, dict: &FixDictionary) -> FixTagLookup {
    let mut lookup = FixTagLookup::from_dictionary(dict);

    if needs_session_merge(key)
        && let Some(session) = get_dictionary(SESSION_KEY)
    {
        merge_lookup(&mut lookup, &session);
    }

    lookup
}

fn build_message_defs(
    messages: &MessageContainer,
    components: &HashMap<String, ComponentDef>,
    name_to_tag: &HashMap<String, u32>,
) -> HashMap<String, MessageDef> {
    let mut map = HashMap::new();
    for msg in &messages.items {
        let (field_order, required) = expand_message_fields(msg, components, name_to_tag, true);
        map.insert(
            msg.msg_type.clone(),
            MessageDef {
                _name: msg.name.clone(),
                _msg_type: msg.msg_type.clone(),
                field_order,
                required,
            },
        );
    }
    map
}

fn expand_message_fields(
    msg: &Message,
    components: &HashMap<String, ComponentDef>,
    name_to_tag: &HashMap<String, u32>,
    include_header_trailer: bool,
) -> (Vec<u32>, Vec<u32>) {
    let mut order = Vec::new();
    let mut required = Vec::new();
    let mut stack = Vec::new();

    if include_header_trailer {
        append_component_fields(
            "Header",
            components,
            name_to_tag,
            &mut stack,
            &mut order,
            &mut required,
        );
    }
    append_field_refs(&msg.fields, name_to_tag, &mut order, &mut required);
    for comp in &msg.components {
        append_component_fields(
            &comp.name,
            components,
            name_to_tag,
            &mut stack,
            &mut order,
            &mut required,
        );
    }
    for group in &msg.groups {
        append_group_fields(
            group,
            components,
            name_to_tag,
            &mut stack,
            &mut order,
            &mut required,
        );
    }

    if include_header_trailer {
        append_component_fields(
            "Trailer",
            components,
            name_to_tag,
            &mut stack,
            &mut order,
            &mut required,
        );
    }

    dedupe(&mut required);
    (order, required)
}

fn append_field_refs(
    refs: &[crate::decoder::schema::FieldRef],
    name_to_tag: &HashMap<String, u32>,
    order: &mut Vec<u32>,
    required: &mut Vec<u32>,
) {
    for field in refs {
        if let Some(tag) = name_to_tag.get(&field.name) {
            order.push(*tag);
            if field.required.as_deref() == Some("Y") {
                required.push(*tag);
            }
        }
    }
}

fn append_component_fields(
    name: &str,
    components: &HashMap<String, ComponentDef>,
    name_to_tag: &HashMap<String, u32>,
    stack: &mut Vec<String>,
    order: &mut Vec<u32>,
    required: &mut Vec<u32>,
) {
    if stack.contains(&name.to_string()) {
        eprintln!("warning: component recursion detected at {name}, skipping nested expansion");
        return;
    }
    let Some(comp) = components.get(name) else {
        return;
    };
    stack.push(name.to_string());

    append_field_refs(&comp.fields, name_to_tag, order, required);
    for sub in &comp.components {
        append_component_fields(&sub.name, components, name_to_tag, stack, order, required);
    }
    for group in &comp.groups {
        append_group_fields(group, components, name_to_tag, stack, order, required);
    }

    stack.pop();
}

fn append_group_fields(
    group: &GroupDef,
    components: &HashMap<String, ComponentDef>,
    name_to_tag: &HashMap<String, u32>,
    stack: &mut Vec<String>,
    order: &mut Vec<u32>,
    required: &mut Vec<u32>,
) {
    append_field_refs(&group.fields, name_to_tag, order, required);
    for comp in &group.components {
        append_component_fields(&comp.name, components, name_to_tag, stack, order, required);
    }
    for sub in &group.groups {
        append_group_fields(sub, components, name_to_tag, stack, order, required);
    }
}

fn dedupe(values: &mut Vec<u32>) {
    let mut seen = HashSet::new();
    values.retain(|v| seen.insert(*v));
}

fn collect_repeatable_tags(
    messages: &MessageContainer,
    components: &HashMap<String, ComponentDef>,
    name_to_tag: &HashMap<String, u32>,
) -> HashSet<u32> {
    let mut repeatable = HashSet::new();
    let mut component_stack = HashSet::new();

    for message in &messages.items {
        for component in &message.components {
            collect_component_repeatables(
                &component.name,
                components,
                name_to_tag,
                &mut repeatable,
                &mut component_stack,
            );
        }
        for group in &message.groups {
            collect_group_repeatables(
                group,
                components,
                name_to_tag,
                &mut repeatable,
                &mut component_stack,
            );
        }
    }

    repeatable
}

fn collect_component_repeatables(
    name: &str,
    components: &HashMap<String, ComponentDef>,
    name_to_tag: &HashMap<String, u32>,
    repeatable: &mut HashSet<u32>,
    stack: &mut HashSet<String>,
) {
    if !stack.insert(name.to_string()) {
        return;
    }
    let Some(comp) = components.get(name) else {
        stack.remove(name);
        return;
    };

    for group in &comp.groups {
        collect_group_repeatables(group, components, name_to_tag, repeatable, stack);
    }
    for child in &comp.components {
        collect_component_repeatables(&child.name, components, name_to_tag, repeatable, stack);
    }

    stack.remove(name);
}

fn collect_group_repeatables(
    group: &GroupDef,
    components: &HashMap<String, ComponentDef>,
    name_to_tag: &HashMap<String, u32>,
    repeatable: &mut HashSet<u32>,
    stack: &mut HashSet<String>,
) {
    if let Some(tag) = name_to_tag.get(&group.name) {
        repeatable.insert(*tag);
    }
    for field in &group.fields {
        if let Some(tag) = name_to_tag.get(&field.name) {
            repeatable.insert(*tag);
        }
    }
    for comp in &group.components {
        collect_component_repeatables(&comp.name, components, name_to_tag, repeatable, stack);
    }
    for sub in &group.groups {
        collect_group_repeatables(sub, components, name_to_tag, repeatable, stack);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_schema_from_default_appl_ver_id() {
        let msg = "8=FIXT.1.1\u{0001}35=D\u{0001}1137=8\u{0001}10=000\u{0001}";
        assert_eq!(detect_schema_key(msg), "FIX50SP1");
    }
}
