//! Pull zone domain tools (design §6, specs/tools/pullzone.md).
//!
//! Implements read and write tools for pull zones: list, get (summary/verbose),
//! create, update, delete, check availability, and reset security key.

use rmcp::handler::server::wrapper::Parameters;
use rmcp::schemars::JsonSchema;
use rmcp::{tool, tool_router};
use serde::Deserialize;
use serde_json::{Map, Value, json};
use url::Url;

use crate::client::BunnyError;
use crate::generated::{FieldType, PULL_ZONE_SETTINGS};
use crate::model::pullzone::{PullZoneListApiResponse, PullZoneSummary};
use crate::server::BunnyServer;
use crate::tools::util::{ToolResult, api_error, invalid, ok_json, ok_unit};

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PullZoneListArgs {
    /// Substring of the pull zone name.
    #[serde(default)]
    pub search: Option<String>,
    /// Page number.
    #[serde(default = "default_page")]
    pub page: u32,
    /// Items per page.
    #[serde(default = "default_per_page")]
    pub per_page: u32,
}

fn default_page() -> u32 {
    1
}

fn default_per_page() -> u32 {
    100
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PullZoneGetArgs {
    /// Pull zone id.
    pub id: u64,
    /// Return the full API object instead of the summary.
    #[serde(default)]
    pub verbose: bool,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PullZoneCheckAvailabilityArgs {
    /// Candidate name.
    pub name: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PullZoneCreateArgs {
    /// Zone name; becomes <name>.b-cdn.net.
    pub name: String,
    /// Origin server URL.
    #[serde(default)]
    pub origin_url: Option<String>,
    /// Use a Bunny storage zone as origin.
    #[serde(default)]
    pub storage_zone_id: Option<u64>,
    /// Pricing tier.
    #[serde(default)]
    pub tier: Option<String>,
    /// Extra settings (same keys as bunny_pullzone_update).
    #[serde(default)]
    pub settings: Option<Map<String, Value>>,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PullZoneUpdateArgs {
    /// Pull zone id.
    pub id: u64,
    /// Partial PullZoneSettingsModel.
    pub settings: Map<String, Value>,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PullZoneDeleteArgs {
    /// Pull zone id.
    pub id: u64,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PullZoneResetSecurityKeyArgs {
    /// Pull zone id.
    pub id: u64,
    /// New key; Bunny generates one if omitted.
    #[serde(default)]
    pub security_key: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct CheckAvailabilityResponse {
    available: Option<bool>,
}

/// Pure Levenshtein distance between two strings.
pub fn levenshtein(a: &str, b: &str) -> usize {
    let b_len = b.chars().count();
    let mut prev: Vec<usize> = (0..=b_len).collect();
    let mut curr = vec![0; b_len + 1];

    for (i, ca) in a.chars().enumerate() {
        curr[0] = i + 1;
        for (j, cb) in b.chars().enumerate() {
            let cost = if ca == cb { 0 } else { 1 };
            curr[j + 1] = (prev[j + 1] + 1).min(curr[j] + 1).min(prev[j] + cost);
        }
        prev.copy_from_slice(&curr);
    }
    prev[b_len]
}

fn value_type_name(v: &Value) -> &'static str {
    match v {
        Value::Null => "null",
        Value::Bool(_) => "bool",
        Value::Number(n) => {
            if n.is_i64() || n.is_u64() {
                "int"
            } else {
                "number"
            }
        }
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}

/// Pure function validating and normalizing pull zone settings using `generated::PULL_ZONE_SETTINGS`.
///
/// 1. Unknown key -> error with up to 3 suggestions (Levenshtein distance <= 3 or case-insensitive match).
/// 2. Case-insensitive key match -> normalize to canonical PascalCase (not an error).
/// 3. Incompatible type -> error `setting "X" expects bool, got string`.
///    For `FieldType::Enum`, accepts variant name (case-insensitive string) and converts to integer.
pub fn validate_settings(settings: &Map<String, Value>) -> Result<Map<String, Value>, Vec<String>> {
    let mut normalized = Map::new();
    let mut errors = Vec::new();

    for (key, val) in settings {
        // Step 1 & 2: Match against PULL_ZONE_SETTINGS
        let matched = match PULL_ZONE_SETTINGS.binary_search_by_key(&key.as_str(), |&(k, _)| k) {
            Ok(idx) => Some(PULL_ZONE_SETTINGS[idx]),
            Err(_) => PULL_ZONE_SETTINGS
                .iter()
                .find(|(k, _)| k.eq_ignore_ascii_case(key))
                .copied(),
        };

        let (canon_name, field_type) = match matched {
            Some(entry) => entry,
            None => {
                // Unknown setting -> find up to 3 suggestions with Levenshtein distance <= 3
                let key_lower = key.to_ascii_lowercase();
                let mut candidates: Vec<(usize, &'static str)> = Vec::new();
                for &(candidate, _) in PULL_ZONE_SETTINGS {
                    let dist = levenshtein(&key_lower, &candidate.to_ascii_lowercase())
                        .min(levenshtein(key, candidate));
                    if dist <= 3 {
                        candidates.push((dist, candidate));
                    }
                }
                candidates.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.cmp(b.1)));
                candidates.dedup_by(|a, b| a.1 == b.1);
                let suggestions: Vec<&'static str> =
                    candidates.into_iter().take(3).map(|(_, s)| s).collect();

                let err = match suggestions.len() {
                    0 => format!("unknown setting \"{key}\""),
                    1 => format!(
                        "unknown setting \"{key}\" (did you mean \"{}\"?)",
                        suggestions[0]
                    ),
                    _ => {
                        let formatted: Vec<String> =
                            suggestions.iter().map(|s| format!("\"{s}\"")).collect();
                        format!(
                            "unknown setting \"{key}\" (did you mean {}?)",
                            formatted.join(", ")
                        )
                    }
                };
                errors.push(err);
                continue;
            }
        };

        // Step 3: Type check against FieldType
        match field_type {
            FieldType::Bool => {
                if val.is_boolean() {
                    normalized.insert(canon_name.to_string(), val.clone());
                } else {
                    errors.push(format!(
                        "setting \"{canon_name}\" expects bool, got {}",
                        value_type_name(val)
                    ));
                }
            }
            FieldType::Int => {
                if val.is_i64() || val.is_u64() {
                    normalized.insert(canon_name.to_string(), val.clone());
                } else {
                    errors.push(format!(
                        "setting \"{canon_name}\" expects int, got {}",
                        value_type_name(val)
                    ));
                }
            }
            FieldType::Number => {
                if val.is_number() {
                    normalized.insert(canon_name.to_string(), val.clone());
                } else {
                    errors.push(format!(
                        "setting \"{canon_name}\" expects number, got {}",
                        value_type_name(val)
                    ));
                }
            }
            FieldType::String => {
                if val.is_string() {
                    normalized.insert(canon_name.to_string(), val.clone());
                } else {
                    errors.push(format!(
                        "setting \"{canon_name}\" expects string, got {}",
                        value_type_name(val)
                    ));
                }
            }
            FieldType::StringArray => {
                if let Some(arr) = val.as_array() {
                    if arr.iter().all(|item| item.is_string()) {
                        normalized.insert(canon_name.to_string(), val.clone());
                    } else {
                        errors.push(format!(
                            "setting \"{canon_name}\" expects string array, got array with non-string elements"
                        ));
                    }
                } else {
                    errors.push(format!(
                        "setting \"{canon_name}\" expects string array, got {}",
                        value_type_name(val)
                    ));
                }
            }
            FieldType::IntArray => {
                if let Some(arr) = val.as_array() {
                    if arr.iter().all(|item| item.is_i64() || item.is_u64()) {
                        normalized.insert(canon_name.to_string(), val.clone());
                    } else {
                        errors.push(format!(
                            "setting \"{canon_name}\" expects int array, got array with non-int elements"
                        ));
                    }
                } else {
                    errors.push(format!(
                        "setting \"{canon_name}\" expects int array, got {}",
                        value_type_name(val)
                    ));
                }
            }
            FieldType::Object => {
                if val.is_object() {
                    normalized.insert(canon_name.to_string(), val.clone());
                } else {
                    errors.push(format!(
                        "setting \"{canon_name}\" expects object, got {}",
                        value_type_name(val)
                    ));
                }
            }
            FieldType::ObjectArray => {
                if let Some(arr) = val.as_array() {
                    if arr.iter().all(|item| item.is_object()) {
                        normalized.insert(canon_name.to_string(), val.clone());
                    } else {
                        errors.push(format!(
                            "setting \"{canon_name}\" expects object array, got array with non-object elements"
                        ));
                    }
                } else {
                    errors.push(format!(
                        "setting \"{canon_name}\" expects object array, got {}",
                        value_type_name(val)
                    ));
                }
            }
            FieldType::Enum(table) => {
                if let Some(s) = val.as_str() {
                    match crate::generated::lookup(table, s) {
                        Some(code) => {
                            normalized.insert(canon_name.to_string(), json!(code));
                        }
                        None => {
                            errors.push(format!(
                                "setting \"{canon_name}\" has invalid enum value \"{s}\""
                            ));
                        }
                    }
                } else if let Some(n) = val.as_u64() {
                    let code = n as u8;
                    if table.iter().any(|&(_, c)| c == code) {
                        normalized.insert(canon_name.to_string(), json!(code));
                    } else {
                        errors.push(format!(
                            "setting \"{canon_name}\" has invalid enum value {code}"
                        ));
                    }
                } else if let Some(n) = val.as_i64() {
                    if (0..=u8::MAX as i64).contains(&n) && table.iter().any(|&(_, c)| c == n as u8)
                    {
                        normalized.insert(canon_name.to_string(), json!(n as u8));
                    } else {
                        errors.push(format!(
                            "setting \"{canon_name}\" has invalid enum value {n}"
                        ));
                    }
                } else {
                    errors.push(format!(
                        "setting \"{canon_name}\" expects enum, got {}",
                        value_type_name(val)
                    ));
                }
            }
        }
    }

    if errors.is_empty() {
        Ok(normalized)
    } else {
        Err(errors)
    }
}

#[tool_router(router = pullzone_router, vis = "pub(crate)")]
impl BunnyServer {
    /// List pull zones (id, name, origin, hostnames). Use `search` to filter by name.
    #[tool(
        name = "bunny_pullzone_list",
        annotations(
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = false,
            open_world_hint = true
        )
    )]
    pub async fn bunny_pullzone_list(
        &self,
        Parameters(args): Parameters<PullZoneListArgs>,
    ) -> ToolResult {
        if args.page < 1 {
            return invalid("page must be >= 1");
        }
        if !(5..=1000).contains(&args.per_page) {
            return invalid("per_page must be between 5 and 1000");
        }

        let mut query = vec![
            ("page", args.page.to_string()),
            ("perPage", args.per_page.to_string()),
            ("includeCertificate", "false".to_string()),
        ];
        if let Some(search) = &args.search
            && !search.is_empty()
        {
            query.push(("search", search.clone()));
        }

        match self
            .client
            .get::<PullZoneListApiResponse>("/pullzone", &query)
            .await
        {
            Ok(resp) => {
                let result = resp.into_response(args.page, args.per_page);
                ok_json(&result)
            }
            Err(e) => api_error(e),
        }
    }

    /// Get one pull zone's configuration summary. Set `verbose` for the full (redacted) object.
    #[tool(
        name = "bunny_pullzone_get",
        annotations(
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = false,
            open_world_hint = true
        )
    )]
    pub async fn bunny_pullzone_get(
        &self,
        Parameters(args): Parameters<PullZoneGetArgs>,
    ) -> ToolResult {
        if args.id == 0 {
            return invalid("id must be >= 1");
        }

        let path = format!("/pullzone/{}", args.id);
        match self.client.get::<Value>(&path, &[]).await {
            Ok(val) => {
                if args.verbose {
                    let redacted = crate::model::redact::redact(val);
                    ok_json(&redacted)
                } else {
                    match PullZoneSummary::from_value(&val) {
                        Ok(summary) => ok_json(&summary),
                        Err(e) => api_error(e),
                    }
                }
            }
            Err(e) => api_error(e),
        }
    }

    /// Check whether a pull zone name is available.
    #[tool(
        name = "bunny_pullzone_check_availability",
        annotations(
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = false,
            open_world_hint = true
        )
    )]
    pub async fn bunny_pullzone_check_availability(
        &self,
        Parameters(args): Parameters<PullZoneCheckAvailabilityArgs>,
    ) -> ToolResult {
        if args.name.trim().is_empty() {
            return invalid("name cannot be empty");
        }

        let body = json!({ "Name": args.name });
        match self
            .client
            .post::<Value>("/pullzone/checkavailability", &[], Some(&body))
            .await
        {
            Ok(val) => {
                let resp: Result<CheckAvailabilityResponse, _> = serde_json::from_value(val);
                match resp {
                    Ok(CheckAvailabilityResponse {
                        available: Some(available),
                    }) => ok_json(&json!({
                        "name": args.name,
                        "available": available
                    })),
                    _ => api_error(BunnyError::Decode(
                        "response missing Available field".to_string(),
                    )),
                }
            }
            Err(e) => api_error(e),
        }
    }

    /// Create a pull zone. Provide `origin_url` or `storage_zone_id`. Takes effect immediately.
    #[tool(
        name = "bunny_pullzone_create",
        annotations(
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = false,
            open_world_hint = true
        )
    )]
    pub async fn bunny_pullzone_create(
        &self,
        Parameters(args): Parameters<PullZoneCreateArgs>,
    ) -> ToolResult {
        if args.name.len() < 3
            || !args
                .name
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
        {
            return invalid("name must match ^[a-z0-9-]{3,}$");
        }

        let has_origin_url = args
            .origin_url
            .as_ref()
            .is_some_and(|s| !s.trim().is_empty());
        let has_storage_id = args.storage_zone_id.is_some_and(|id| id > 0);

        if has_origin_url == has_storage_id {
            return invalid("exactly one of origin_url or storage_zone_id must be provided");
        }

        if let Some(ref url_str) = args.origin_url {
            match Url::parse(url_str) {
                Ok(parsed) => {
                    let scheme = parsed.scheme();
                    let host_empty = parsed.host_str().is_none_or(|h| h.is_empty());
                    if (scheme != "http" && scheme != "https") || host_empty {
                        return invalid("origin_url must have http or https scheme and a host");
                    }
                }
                Err(e) => return invalid(format!("invalid origin_url: {e}")),
            }
        }

        if let Some(id) = args.storage_zone_id
            && id == 0
        {
            return invalid("storage_zone_id must be >= 1");
        }

        let tier_type = match args.tier.as_deref().unwrap_or("standard") {
            t if t.eq_ignore_ascii_case("standard") => 0,
            t if t.eq_ignore_ascii_case("volume") => 1,
            _ => return invalid("tier must be \"standard\" or \"volume\""),
        };

        let normalized_settings = if let Some(ref settings_map) = args.settings {
            for key in settings_map.keys() {
                let lower = key.to_ascii_lowercase();
                if lower == "name"
                    || lower == "originurl"
                    || lower == "storagezoneid"
                    || lower == "type"
                    || lower == "origintype"
                {
                    return invalid(format!("settings cannot contain \"{key}\""));
                }
            }
            match validate_settings(settings_map) {
                Ok(norm) => norm,
                Err(errors) => return invalid(errors.join("; ")),
            }
        } else {
            Map::new()
        };

        let mut body = Map::new();
        body.insert("Name".to_string(), json!(args.name));
        body.insert("Type".to_string(), json!(tier_type));
        if let Some(ref origin_url) = args.origin_url {
            body.insert("OriginUrl".to_string(), json!(origin_url));
        } else if let Some(storage_id) = args.storage_zone_id {
            body.insert("StorageZoneId".to_string(), json!(storage_id));
            body.insert("OriginType".to_string(), json!(2));
        }
        for (k, v) in normalized_settings {
            body.insert(k, v);
        }

        match self
            .client
            .post::<Value>("/pullzone", &[], Some(&Value::Object(body)))
            .await
        {
            Ok(val) => {
                if val.is_object() && (val.get("Id").is_some() || val.get("id").is_some()) {
                    match PullZoneSummary::from_value(&val) {
                        Ok(summary) => ok_json(&summary),
                        Err(e) => api_error(e),
                    }
                } else {
                    ok_unit(json!({
                        "name": args.name,
                        "hint": format!("use bunny_pullzone_list search={} to get the id", args.name),
                    }))
                }
            }
            Err(e) => api_error(e),
        }
    }

    /// Update pull zone settings. `settings` is a partial object using Bunny's PascalCase keys (e.g. {"EnableSmartCache":true,"CacheControlMaxAgeOverride":3600}).
    #[tool(
        name = "bunny_pullzone_update",
        annotations(
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = true
        )
    )]
    pub async fn bunny_pullzone_update(
        &self,
        Parameters(args): Parameters<PullZoneUpdateArgs>,
    ) -> ToolResult {
        if args.id == 0 {
            return invalid("id must be >= 1");
        }
        if args.settings.is_empty() {
            return invalid("settings must contain at least 1 key");
        }

        let normalized_settings = match validate_settings(&args.settings) {
            Ok(s) => s,
            Err(errors) => return invalid(errors.join("; ")),
        };

        let mut updated_keys: Vec<String> = normalized_settings.keys().cloned().collect();
        updated_keys.sort();

        let path = format!("/pullzone/{}", args.id);
        let body = Value::Object(normalized_settings);

        match self.client.post::<Value>(&path, &[], Some(&body)).await {
            Ok(val) => {
                if val.is_object() && (val.get("Id").is_some() || val.get("id").is_some()) {
                    match PullZoneSummary::from_value(&val) {
                        Ok(summary) => ok_json(&summary),
                        Err(e) => api_error(e),
                    }
                } else {
                    match self.client.get::<Value>(&path, &[]).await {
                        Ok(get_val) => match PullZoneSummary::from_value(&get_val) {
                            Ok(summary) => {
                                let mut out_val = serde_json::to_value(&summary).map_err(|e| {
                                    rmcp::ErrorData::internal_error(
                                        format!("failed to serialize summary: {e}"),
                                        None,
                                    )
                                })?;
                                if let Value::Object(ref mut map) = out_val {
                                    map.insert("updated".to_string(), json!(updated_keys));
                                }
                                ok_json(&out_val)
                            }
                            Err(e) => api_error(e),
                        },
                        Err(e) => api_error(e),
                    }
                }
            }
            Err(e) => api_error(e),
        }
    }

    /// Permanently delete a pull zone and its configuration. Cannot be undone.
    #[tool(
        name = "bunny_pullzone_delete",
        annotations(
            read_only_hint = false,
            destructive_hint = true,
            idempotent_hint = true,
            open_world_hint = true
        )
    )]
    pub async fn bunny_pullzone_delete(
        &self,
        Parameters(args): Parameters<PullZoneDeleteArgs>,
    ) -> ToolResult {
        if args.id == 0 {
            return invalid("id must be >= 1");
        }

        let path = format!("/pullzone/{}", args.id);
        match self.client.delete::<Value>(&path, None).await {
            Ok(_) => ok_unit(json!({
                "deleted_id": args.id
            })),
            Err(e) => api_error(e),
        }
    }

    /// Reset the token-authentication security key of a pull zone. Existing signed URLs stop working.
    #[tool(
        name = "bunny_pullzone_reset_security_key",
        annotations(
            read_only_hint = false,
            destructive_hint = true,
            idempotent_hint = false,
            open_world_hint = true
        )
    )]
    pub async fn bunny_pullzone_reset_security_key(
        &self,
        Parameters(args): Parameters<PullZoneResetSecurityKeyArgs>,
    ) -> ToolResult {
        if args.id == 0 {
            return invalid("id must be >= 1");
        }
        if let Some(ref key) = args.security_key
            && key.trim().is_empty()
        {
            return invalid("security_key cannot be empty");
        }

        let path = format!("/pullzone/{}/resetSecurityKey", args.id);
        let body = match &args.security_key {
            Some(key) => json!({ "SecurityKey": key }),
            None => json!({}),
        };

        match self.client.post::<Value>(&path, &[], Some(&body)).await {
            Ok(_) => ok_unit(json!({
                "id": args.id,
                "note": "new key is visible in the Bunny dashboard"
            })),
            Err(e) => api_error(e),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_levenshtein_distance() {
        assert_eq!(levenshtein("", ""), 0);
        assert_eq!(levenshtein("a", ""), 1);
        assert_eq!(levenshtein("", "a"), 1);
        assert_eq!(levenshtein("abc", "abc"), 0);
        assert_eq!(levenshtein("kitten", "sitting"), 3);
        assert_eq!(levenshtein("EnableSmartCach", "EnableSmartCache"), 1);
    }

    #[test]
    fn test_validate_settings_success_and_normalization() {
        let mut map = Map::new();
        map.insert("enablesmartcache".to_string(), json!(true));
        map.insert("CacheControlMaxAgeOverride".to_string(), json!(3600));
        map.insert("TlsSecurityLevel".to_string(), json!("Compatible"));

        let res = validate_settings(&map).expect("validation should succeed");
        assert_eq!(res.get("EnableSmartCache"), Some(&json!(true)));
        assert_eq!(res.get("CacheControlMaxAgeOverride"), Some(&json!(3600)));
        assert_eq!(res.get("TlsSecurityLevel"), Some(&json!(1)));
    }

    #[test]
    fn test_validate_settings_unknown_with_suggestion() {
        let mut map = Map::new();
        map.insert("EnableSmartCach".to_string(), json!(true));

        let errs = validate_settings(&map).unwrap_err();
        assert_eq!(errs.len(), 1);
        assert_eq!(
            errs[0],
            "unknown setting \"EnableSmartCach\" (did you mean \"EnableSmartCache\"?)"
        );
    }

    #[test]
    fn test_validate_settings_unknown_no_suggestion() {
        let mut map = Map::new();
        map.insert("XyzTotallyUnknownKey999".to_string(), json!(true));

        let errs = validate_settings(&map).unwrap_err();
        assert_eq!(errs.len(), 1);
        assert_eq!(errs[0], "unknown setting \"XyzTotallyUnknownKey999\"");
    }

    #[test]
    fn test_validate_settings_incompatible_type() {
        let mut map = Map::new();
        map.insert("EnableSmartCache".to_string(), json!("true"));
        map.insert("CacheControlMaxAgeOverride".to_string(), json!(true));

        let errs = validate_settings(&map).unwrap_err();
        assert!(
            errs.iter()
                .any(|e| e == "setting \"EnableSmartCache\" expects bool, got string")
        );
        assert!(
            errs.iter()
                .any(|e| e == "setting \"CacheControlMaxAgeOverride\" expects int, got bool")
        );
    }

    #[test]
    fn test_validate_settings_enum_variants() {
        let mut map = Map::new();
        map.insert("TlsSecurityLevel".to_string(), json!("modernonly"));
        let res = validate_settings(&map).unwrap();
        assert_eq!(res.get("TlsSecurityLevel"), Some(&json!(2)));

        let mut invalid_map = Map::new();
        invalid_map.insert("TlsSecurityLevel".to_string(), json!("NonExistent"));
        let errs = validate_settings(&invalid_map).unwrap_err();
        assert!(errs[0].contains("has invalid enum value \"NonExistent\""));
    }
}
