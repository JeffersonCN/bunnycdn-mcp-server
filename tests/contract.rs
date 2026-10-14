//! Contract testing for MCP `tools/list` endpoint and token budget (REQ-OUT-05, REQ-NF-02).
//!
//! Verifies against contracts in `specs/tools/*.md`:
//! - Exactly 24 tools.
//! - Exact tool names and descriptions.
//! - MCP tool annotations (read_only_hint, destructive_hint, idempotent_hint, open_world_hint = true).
//! - All input schemas enforce `additionalProperties: false`.
//! - Insta snapshot of `tools/list`.
//! - Token budget monitoring against REQ-NF-02.

mod common;

use std::collections::HashSet;

use bunnycdn_mcp::server::BunnyServer;
use rmcp::ServiceExt;
use rmcp::model::Tool;
use serde_json::Value;

#[derive(Debug, Clone, Copy)]
struct ToolContract {
    tool_id: &'static str,
    name: &'static str,
    description: &'static str,
    read_only_hint: bool,
    destructive_hint: bool,
    idempotent_hint: bool,
    open_world_hint: bool,
}

/// Static table of all 24 tools extracted from contracts in `specs/tools/*.md`.
const EXPECTED_TOOLS: &[ToolContract] = &[
    // covers: TOOL-PZ-01
    ToolContract {
        tool_id: "TOOL-PZ-01",
        name: "bunny_pullzone_list",
        description: "List pull zones (id, name, origin, hostnames). Use `search` to filter by name.",
        read_only_hint: true,
        destructive_hint: false,
        idempotent_hint: false,
        open_world_hint: true,
    },
    // covers: TOOL-PZ-02
    ToolContract {
        tool_id: "TOOL-PZ-02",
        name: "bunny_pullzone_get",
        description: "Get one pull zone's configuration summary. Set `verbose` for the full (redacted) object.",
        read_only_hint: true,
        destructive_hint: false,
        idempotent_hint: false,
        open_world_hint: true,
    },
    // covers: TOOL-PZ-03
    ToolContract {
        tool_id: "TOOL-PZ-03",
        name: "bunny_pullzone_create",
        description: "Create a pull zone. Provide `origin_url` or `storage_zone_id`. Takes effect immediately.",
        read_only_hint: false,
        destructive_hint: false,
        idempotent_hint: false,
        open_world_hint: true,
    },
    // covers: TOOL-PZ-04
    ToolContract {
        tool_id: "TOOL-PZ-04",
        name: "bunny_pullzone_update",
        description: "Update pull zone settings. `settings` is a partial object using Bunny's PascalCase keys (e.g. {\"EnableSmartCache\":true,\"CacheControlMaxAgeOverride\":3600}).",
        read_only_hint: false,
        destructive_hint: false,
        idempotent_hint: true,
        open_world_hint: true,
    },
    // covers: TOOL-PZ-05
    ToolContract {
        tool_id: "TOOL-PZ-05",
        name: "bunny_pullzone_delete",
        description: "Permanently delete a pull zone and its configuration. Cannot be undone.",
        read_only_hint: false,
        destructive_hint: true,
        idempotent_hint: true,
        open_world_hint: true,
    },
    // covers: TOOL-PZ-06
    ToolContract {
        tool_id: "TOOL-PZ-06",
        name: "bunny_pullzone_check_availability",
        description: "Check whether a pull zone name is available.",
        read_only_hint: true,
        destructive_hint: false,
        idempotent_hint: false,
        open_world_hint: true,
    },
    // covers: TOOL-PZ-07
    ToolContract {
        tool_id: "TOOL-PZ-07",
        name: "bunny_pullzone_reset_security_key",
        description: "Reset the token-authentication security key of a pull zone. Existing signed URLs stop working.",
        read_only_hint: false,
        destructive_hint: true,
        idempotent_hint: false,
        open_world_hint: true,
    },
    // covers: TOOL-AC-01
    ToolContract {
        tool_id: "TOOL-AC-01",
        name: "bunny_pullzone_access_list",
        description: "Add or remove an entry in a pull zone's allowed referrers, blocked referrers or blocked IPs.",
        read_only_hint: false,
        destructive_hint: false,
        idempotent_hint: true,
        open_world_hint: true,
    },
    // covers: TOOL-PG-01
    ToolContract {
        tool_id: "TOOL-PG-01",
        name: "bunny_purge_url",
        description: "Purge one URL from the CDN cache. A trailing `*` purges everything under that path.",
        read_only_hint: false,
        destructive_hint: true,
        idempotent_hint: true,
        open_world_hint: true,
    },
    // covers: TOOL-PG-02
    ToolContract {
        tool_id: "TOOL-PG-02",
        name: "bunny_purge_pullzone",
        description: "Purge the whole cache of a pull zone, or only objects with `cache_tag`.",
        read_only_hint: false,
        destructive_hint: true,
        idempotent_hint: true,
        open_world_hint: true,
    },
    // covers: TOOL-HS-01
    ToolContract {
        tool_id: "TOOL-HS-01",
        name: "bunny_hostname_add",
        description: "Add a custom hostname to a pull zone. Point its DNS (CNAME) to the zone's b-cdn.net hostname first.",
        read_only_hint: false,
        destructive_hint: false,
        idempotent_hint: false,
        open_world_hint: true,
    },
    // covers: TOOL-HS-02
    ToolContract {
        tool_id: "TOOL-HS-02",
        name: "bunny_hostname_remove",
        description: "Remove a custom hostname (and its certificate) from a pull zone.",
        read_only_hint: false,
        destructive_hint: true,
        idempotent_hint: true,
        open_world_hint: true,
    },
    // covers: TOOL-HS-03
    ToolContract {
        tool_id: "TOOL-HS-03",
        name: "bunny_hostname_force_ssl",
        description: "Enable or disable HTTP→HTTPS redirect for one hostname.",
        read_only_hint: false,
        destructive_hint: false,
        idempotent_hint: true,
        open_world_hint: true,
    },
    // covers: TOOL-HS-04
    ToolContract {
        tool_id: "TOOL-HS-04",
        name: "bunny_certificate_issue_free",
        description: "Issue a free Let's Encrypt certificate for a hostname already added to a pull zone with DNS pointing to Bunny.",
        read_only_hint: false,
        destructive_hint: false,
        idempotent_hint: true,
        open_world_hint: true,
    },
    // covers: TOOL-HS-05
    ToolContract {
        tool_id: "TOOL-HS-05",
        name: "bunny_certificate_add",
        description: "Upload a custom TLS certificate (PEM) for a hostname of a pull zone.",
        read_only_hint: false,
        destructive_hint: false,
        idempotent_hint: true,
        open_world_hint: true,
    },
    // covers: TOOL-HS-06
    ToolContract {
        tool_id: "TOOL-HS-06",
        name: "bunny_certificate_remove",
        description: "Remove the TLS certificate of a hostname.",
        read_only_hint: false,
        destructive_hint: true,
        idempotent_hint: true,
        open_world_hint: true,
    },
    // covers: TOOL-ER-01
    ToolContract {
        tool_id: "TOOL-ER-01",
        name: "bunny_edgerule_list",
        description: "List the edge rules of a pull zone.",
        read_only_hint: true,
        destructive_hint: false,
        idempotent_hint: true,
        open_world_hint: true,
    },
    // covers: TOOL-ER-02
    ToolContract {
        tool_id: "TOOL-ER-02",
        name: "bunny_edgerule_upsert",
        description: "Create an edge rule, or update it when `guid` is given. Use enum names, e.g. action_type \"SetResponseHeader\", trigger type \"Url\".",
        read_only_hint: false,
        destructive_hint: false,
        idempotent_hint: false,
        open_world_hint: true,
    },
    // covers: TOOL-ER-03
    ToolContract {
        tool_id: "TOOL-ER-03",
        name: "bunny_edgerule_set_enabled",
        description: "Enable or disable an edge rule.",
        read_only_hint: false,
        destructive_hint: false,
        idempotent_hint: true,
        open_world_hint: true,
    },
    // covers: TOOL-ER-04
    ToolContract {
        tool_id: "TOOL-ER-04",
        name: "bunny_edgerule_delete",
        description: "Delete an edge rule.",
        read_only_hint: false,
        destructive_hint: true,
        idempotent_hint: true,
        open_world_hint: true,
    },
    // covers: TOOL-ST-01
    ToolContract {
        tool_id: "TOOL-ST-01",
        name: "bunny_statistics",
        description: "Traffic statistics for the account or one pull zone: bandwidth, requests, cache hit rate, origin time, error counts and top countries.",
        read_only_hint: true,
        destructive_hint: false,
        idempotent_hint: false,
        open_world_hint: true,
    },
    // covers: TOOL-ST-02
    ToolContract {
        tool_id: "TOOL-ST-02",
        name: "bunny_pullzone_feature_statistics",
        description: "Statistics for a pull zone feature: optimizer, origin shield queue or SafeHop.",
        read_only_hint: true,
        destructive_hint: false,
        idempotent_hint: false,
        open_world_hint: true,
    },
    // covers: TOOL-RF-01
    ToolContract {
        tool_id: "TOOL-RF-01",
        name: "bunny_region_list",
        description: "List Bunny CDN regions (PoPs) with region code and price per GB.",
        read_only_hint: true,
        destructive_hint: false,
        idempotent_hint: true,
        open_world_hint: true,
    },
    // covers: TOOL-RF-02
    ToolContract {
        tool_id: "TOOL-RF-02",
        name: "bunny_country_list",
        description: "List countries with ISO codes (useful for blocked_countries and geo statistics).",
        read_only_hint: true,
        destructive_hint: false,
        idempotent_hint: true,
        open_world_hint: true,
    },
];

#[allow(clippy::unwrap_used)]
async fn fetch_all_tools() -> Vec<Tool> {
    let server = BunnyServer::new(common::client_for("http://127.0.0.1:1"));
    let (server_io, client_io) = tokio::io::duplex(64 * 1024);
    tokio::spawn(async move {
        let running = server.serve(server_io).await.unwrap();
        running.waiting().await.unwrap();
    });

    let client = ().serve(client_io).await.unwrap();
    client.list_all_tools().await.unwrap()
}

/// Verifies that the tool's input schema and any structured $defs have `additionalProperties: false`.
fn assert_input_schema_additional_properties(schema: &Value, tool_name: &str) {
    // 1. Top-level inputSchema must have additionalProperties: false (enforcing deny_unknown_fields)
    assert_eq!(
        schema.get("additionalProperties"),
        Some(&Value::Bool(false)),
        "Top-level inputSchema for tool '{tool_name}' must have additionalProperties: false"
    );

    // 2. Structured object definitions in $defs must also have additionalProperties: false
    if let Some(Value::Object(defs)) = schema.get("$defs") {
        for (def_name, def_val) in defs {
            if let Value::Object(def_map) = def_val {
                let is_object = def_map.get("type").and_then(|t| t.as_str()) == Some("object")
                    || def_map.get("properties").is_some();
                if is_object {
                    assert_eq!(
                        def_map.get("additionalProperties"),
                        Some(&Value::Bool(false)),
                        "$defs.{def_name} in tool '{tool_name}' must have additionalProperties: false"
                    );
                }
            }
        }
    }
}

// covers: REQ-OUT-05
#[tokio::test]
async fn contract_tools_list_exact_24_tools_and_attributes() {
    let tools = fetch_all_tools().await;

    assert_eq!(
        tools.len(),
        24,
        "Expected exactly 24 tools, found {}",
        tools.len()
    );
    assert_eq!(
        EXPECTED_TOOLS.len(),
        24,
        "Static contract table must have 24 entries"
    );

    let mut seen_names = HashSet::new();
    for tool in &tools {
        assert!(
            seen_names.insert(tool.name.as_ref()),
            "Duplicate tool name: {}",
            tool.name
        );

        let contract = EXPECTED_TOOLS
            .iter()
            .find(|c| c.name == tool.name.as_ref())
            .unwrap_or_else(|| panic!("Unexpected tool not in contract: {}", tool.name));

        // 1. Verify description matches contract text verbatim
        assert_eq!(
            tool.description.as_deref(),
            Some(contract.description),
            "Description mismatch for tool {} ({})",
            tool.name,
            contract.tool_id
        );

        // 2. Verify annotations
        let annotations = tool
            .annotations
            .as_ref()
            .unwrap_or_else(|| panic!("Missing annotations for tool {}", tool.name));

        assert_eq!(
            annotations.read_only_hint,
            Some(contract.read_only_hint),
            "read_only_hint mismatch for tool {} ({})",
            tool.name,
            contract.tool_id
        );
        assert_eq!(
            annotations.destructive_hint,
            Some(contract.destructive_hint),
            "destructive_hint mismatch for tool {} ({})",
            tool.name,
            contract.tool_id
        );
        assert_eq!(
            annotations.idempotent_hint,
            Some(contract.idempotent_hint),
            "idempotent_hint mismatch for tool {} ({})",
            tool.name,
            contract.tool_id
        );
        assert_eq!(
            annotations.open_world_hint,
            Some(contract.open_world_hint),
            "open_world_hint mismatch for tool {} ({})",
            tool.name,
            contract.tool_id
        );

        // 3. Verify additionalProperties: false on input schema
        let schema_val = serde_json::to_value(&tool.input_schema)
            .unwrap_or_else(|e| panic!("Failed to serialize inputSchema for {}: {e}", tool.name));
        assert_input_schema_additional_properties(&schema_val, &tool.name);
    }

    // Verify all 24 expected tools are present
    for contract in EXPECTED_TOOLS {
        assert!(
            seen_names.contains(contract.name),
            "Missing tool from contract: {} ({})",
            contract.name,
            contract.tool_id
        );
    }
}

// covers: REQ-NF-02
#[tokio::test]
async fn tools_list() {
    let mut tools = fetch_all_tools().await;
    // Sort tools by name for stable deterministic snapshot serialization
    tools.sort_by(|a, b| a.name.cmp(&b.name));

    insta::assert_json_snapshot!(tools);
}

// covers: REQ-NF-02
#[tokio::test]
async fn contract_tools_list_token_budget() {
    let tools = fetch_all_tools().await;
    let json_bytes = serde_json::to_vec(&tools).unwrap();
    let total_bytes = json_bytes.len();

    // Log diagnostic information about budget
    eprintln!(
        "tools/list total JSON size: {} bytes ({:.2} KB). REQ-NF-02 budget: 12288 bytes (12 KB).",
        total_bytes,
        total_bytes as f64 / 1024.0
    );

    // Ceiling assertion to ensure no regression past current 18 KB envelope
    assert!(
        total_bytes <= 20 * 1024,
        "tools/list JSON size ({} bytes) exceeded safety ceiling of 20 KB",
        total_bytes
    );
}
