//! Generates `src/generated.rs` from the vendored OpenAPI spec (design §9).

use crate::sha256;
use serde_json::Value;
use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

const SPEC: &str = "specs/api/bunny-core.openapi.json";
const OUT: &str = "src/generated.rs";

/// (schema name, const name) for the tables fixed by the design.
const NAMED_ENUMS: &[(&str, &str)] = &[
    ("EdgeRuleActionType", "EDGE_RULE_ACTION_TYPES"),
    ("TriggerTypes", "EDGE_RULE_TRIGGER_TYPES"),
    ("TriggerMatchingTypes", "TRIGGER_MATCHING_TYPES"),
    ("PatternMatchingTypes", "PATTERN_MATCHING_TYPES"),
    ("PullZoneType", "PULL_ZONE_TYPES"),
    ("PullZoneOriginType", "PULL_ZONE_ORIGIN_TYPES"),
];

pub fn run(check: bool) -> ExitCode {
    match try_run(check) {
        Ok(code) => code,
        Err(e) => {
            eprintln!("xtask codegen: {e}");
            ExitCode::FAILURE
        }
    }
}

fn root() -> PathBuf {
    // xtask lives in <root>/xtask.
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_default()
}

fn try_run(check: bool) -> Result<ExitCode, String> {
    let root = root();
    let raw = std::fs::read(root.join(SPEC)).map_err(|e| format!("read {SPEC}: {e}"))?;
    let spec: Value = serde_json::from_slice(&raw).map_err(|e| format!("parse {SPEC}: {e}"))?;
    let source = format_rust(&generate(&spec, &sha256::hex_digest(&raw))?)?;

    let out = root.join(OUT);
    if check {
        let current = std::fs::read_to_string(&out).unwrap_or_default();
        if current == source {
            return Ok(ExitCode::SUCCESS);
        }
        eprintln!("{OUT} is out of date; run `cargo xtask codegen` and commit the result");
        return Ok(ExitCode::FAILURE);
    }
    std::fs::write(&out, source).map_err(|e| format!("write {OUT}: {e}"))?;
    Ok(ExitCode::SUCCESS)
}

fn format_rust(src: &str) -> Result<String, String> {
    let dir = std::env::temp_dir().join(format!("xtask-codegen-{}", std::process::id()));
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let file = dir.join("generated.rs");
    std::fs::write(&file, src).map_err(|e| e.to_string())?;
    let status = Command::new("rustfmt")
        .args(["--edition", "2024"])
        .arg(&file)
        .status()
        .map_err(|e| format!("run rustfmt: {e}"))?;
    let formatted = std::fs::read_to_string(&file).map_err(|e| e.to_string());
    let _ = std::fs::remove_dir_all(&dir);
    if !status.success() {
        return Err("rustfmt failed on generated source".into());
    }
    formatted
}

fn schema<'a>(spec: &'a Value, name: &str) -> Result<&'a Value, String> {
    spec.pointer(&format!("/components/schemas/{name}"))
        .ok_or_else(|| format!("schema {name} not found"))
}

fn enum_entries(spec: &Value, name: &str) -> Result<Vec<(String, u8)>, String> {
    let s = schema(spec, name)?;
    let names = s["x-enumNames"]
        .as_array()
        .ok_or_else(|| format!("{name}: missing x-enumNames"))?;
    let values = s["enum"]
        .as_array()
        .ok_or_else(|| format!("{name}: missing enum"))?;
    if names.len() != values.len() {
        return Err(format!("{name}: x-enumNames/enum length mismatch"));
    }
    names
        .iter()
        .zip(values)
        .map(|(n, v)| {
            let n = n.as_str().ok_or_else(|| format!("{name}: bad enum name"))?;
            let v = v
                .as_u64()
                .and_then(|v| u8::try_from(v).ok())
                .ok_or_else(|| format!("{name}: enum value out of u8 range"))?;
            Ok((n.to_string(), v))
        })
        .collect()
}

/// `PullZoneLogFormat` -> `PULL_ZONE_LOG_FORMAT`.
fn screaming_snake(name: &str) -> String {
    let chars: Vec<char> = name.chars().collect();
    let mut out = String::new();
    for (i, c) in chars.iter().enumerate() {
        let prev_lower = i > 0 && (chars[i - 1].is_lowercase() || chars[i - 1].is_ascii_digit());
        let acronym_end = i > 0
            && chars[i - 1].is_uppercase()
            && c.is_uppercase()
            && chars.get(i + 1).is_some_and(|n| n.is_lowercase());
        if c.is_uppercase() && (prev_lower || acronym_end) {
            out.push('_');
        }
        out.push(c.to_ascii_uppercase());
    }
    out
}

fn enum_ref(prop: &Value) -> Option<&str> {
    let r = prop["oneOf"].as_array()?.first()?["$ref"].as_str()?;
    r.strip_prefix("#/components/schemas/")
}

enum Field {
    Plain(&'static str),
    Enum(String),
}

fn classify(spec: &Value, key: &str, prop: &Value) -> Result<Field, String> {
    if let Some(target) = enum_ref(prop) {
        let is_enum = schema(spec, target)?.get("enum").is_some();
        return Ok(if is_enum {
            Field::Enum(target.to_string())
        } else {
            Field::Plain("Object")
        });
    }
    if prop.get("$ref").is_some() {
        return Ok(Field::Plain("Object"));
    }
    match prop["type"].as_str() {
        Some("boolean") => Ok(Field::Plain("Bool")),
        Some("integer") => Ok(Field::Plain("Int")),
        Some("number") => Ok(Field::Plain("Number")),
        Some("string") => Ok(Field::Plain("String")),
        Some("object") => Ok(Field::Plain("Object")),
        Some("array") => {
            let items = &prop["items"];
            if items.get("$ref").is_some() || items["type"] == "object" {
                return Ok(Field::Plain("ObjectArray"));
            }
            match items["type"].as_str() {
                Some("string") => Ok(Field::Plain("StringArray")),
                Some("integer") => Ok(Field::Plain("IntArray")),
                other => Err(format!("{key}: unsupported array items {other:?}")),
            }
        }
        other => Err(format!("{key}: unsupported type {other:?}")),
    }
}

fn emit_table(out: &mut String, const_name: &str, entries: &[(String, u8)]) {
    let _ = writeln!(out, "pub const {const_name}: &[(&str, u8)] = &[");
    for (n, v) in entries {
        let _ = writeln!(out, "    (\"{n}\", {v}),");
    }
    out.push_str("];\n\n");
}

fn generate(spec: &Value, sha: &str) -> Result<String, String> {
    let mut out = String::new();
    let _ = writeln!(
        out,
        "// @generated by `cargo xtask codegen` from {SPEC} — DO NOT EDIT"
    );
    let _ = writeln!(out, "pub const SPEC_SHA256: &str = \"{sha}\";\n");

    let mut tables: BTreeMap<String, String> = BTreeMap::new(); // schema -> const name
    for (schema_name, const_name) in NAMED_ENUMS {
        emit_table(&mut out, const_name, &enum_entries(spec, schema_name)?);
        tables.insert((*schema_name).to_string(), (*const_name).to_string());
    }

    let props = schema(spec, "PullZoneSettingsModel")?["properties"]
        .as_object()
        .ok_or("PullZoneSettingsModel has no properties")?;
    let mut fields = Vec::new();
    for (key, prop) in props {
        fields.push((key.as_str(), classify(spec, key, prop)?));
    }
    fields.sort_by(|a, b| a.0.cmp(b.0));

    // Tables for enums referenced by settings that the design does not name.
    for (_, f) in &fields {
        if let Field::Enum(target) = f
            && !tables.contains_key(target)
        {
            let const_name = screaming_snake(target);
            emit_table(&mut out, &const_name, &enum_entries(spec, target)?);
            tables.insert(target.clone(), const_name);
        }
    }

    out.push_str(
        "#[derive(Clone, Copy, Debug, PartialEq, Eq)]\n\
         pub enum FieldType {\n    Bool,\n    Int,\n    Number,\n    String,\n    StringArray,\n    \
         IntArray,\n    Object,\n    ObjectArray,\n    Enum(&'static [(&'static str, u8)]),\n}\n\n",
    );
    out.push_str(
        "/// Keys accepted by POST /pullzone/{id} (PullZoneSettingsModel), sorted for binary search.\n",
    );
    out.push_str("pub const PULL_ZONE_SETTINGS: &[(&str, FieldType)] = &[\n");
    for (key, f) in &fields {
        let ty = match f {
            Field::Plain(t) => format!("FieldType::{t}"),
            Field::Enum(target) => format!("FieldType::Enum({})", tables[target]),
        };
        let _ = writeln!(out, "    (\"{key}\", {ty}),");
    }
    out.push_str("];\n\n");

    out.push_str(
        "/// Case-insensitive name -> value lookup.\n\
         pub fn lookup(table: &[(&str, u8)], name: &str) -> Option<u8> {\n    \
         table\n        .iter()\n        .find(|(n, _)| n.eq_ignore_ascii_case(name))\n        \
         .map(|&(_, v)| v)\n}\n\n\
         /// Value -> canonical name lookup.\n\
         pub fn name_of(table: &[(&'static str, u8)], v: u8) -> Option<&'static str> {\n    \
         table.iter().find(|&&(_, x)| x == v).map(|&(n, _)| n)\n}\n\n",
    );

    out.push_str(&format!(
        "#[cfg(test)]\nmod tests {{\n    use super::*;\n\n    \
         #[test]\n    fn table_sizes() {{\n        \
         assert_eq!(EDGE_RULE_ACTION_TYPES.len(), 38);\n        \
         assert_eq!(EDGE_RULE_TRIGGER_TYPES.len(), 14);\n        \
         assert_eq!(PULL_ZONE_SETTINGS.len(), {});\n    }}\n\n    \
         #[test]\n    fn settings_are_sorted() {{\n        \
         assert!(PULL_ZONE_SETTINGS.windows(2).all(|w| w[0].0 < w[1].0));\n    }}\n\n    \
         #[test]\n    fn lookup_is_case_insensitive() {{\n        \
         assert_eq!(lookup(EDGE_RULE_ACTION_TYPES, \"setresponseheader\"), Some(5));\n        \
         assert_eq!(lookup(EDGE_RULE_ACTION_TYPES, \"nope\"), None);\n        \
         assert_eq!(name_of(EDGE_RULE_ACTION_TYPES, 5), Some(\"SetResponseHeader\"));\n    }}\n}}\n",
        fields.len()
    ));
    Ok(out)
}
