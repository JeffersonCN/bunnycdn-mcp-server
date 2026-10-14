//! Result helpers shared by every tool (design §5, REQ-OUT-*, C4).
//!
//! Minimal tool, for domain tasks to copy (replace the whole stub in `tools/<domain>.rs`):
//!
//! ```ignore
//! use rmcp::handler::server::wrapper::Parameters;
//! use rmcp::{tool, tool_router};
//! use schemars::JsonSchema;
//! use serde::Deserialize;
//! use serde_json::{Value, json};
//!
//! use crate::server::BunnyServer;
//! use crate::tools::util::{ToolResult, api_error, invalid, ok_unit};
//!
//! #[derive(Debug, Deserialize, JsonSchema)]
//! #[serde(deny_unknown_fields)]
//! pub struct PurgeZoneArgs {
//!     /// Pull zone id.
//!     pub pull_zone_id: u64,
//! }
//!
//! #[tool_router(router = purge_router, vis = "pub(crate)")]
//! impl BunnyServer {
//!     /// Purge the whole cache of a pull zone. Takes effect immediately.
//!     #[tool(
//!         name = "bunny_purge_zone",
//!         annotations(
//!             read_only_hint = false,
//!             destructive_hint = true,
//!             idempotent_hint = true,
//!             open_world_hint = true
//!         )
//!     )]
//!     pub async fn bunny_purge_zone(&self, Parameters(args): Parameters<PurgeZoneArgs>) -> ToolResult {
//!         if args.pull_zone_id == 0 {
//!             return invalid("pull_zone_id must be >= 1");
//!         }
//!         let path = format!("/pullzone/{}/purgeCache", args.pull_zone_id);
//!         match self.client.post::<Value>(&path, &[], None).await {
//!             Ok(_) => ok_unit(json!({ "pull_zone_id": args.pull_zone_id })),
//!             Err(e) => api_error(e),
//!         }
//!     }
//! }
//! ```
//!
//! Test it with the harness in `tests/common/mod.rs`:
//! `srv.bunny_purge_zone(Parameters(args)).await.unwrap()` plus a wiremock mock that matches
//! `header("AccessKey", "test-key")`.

use rmcp::ErrorData;
use rmcp::model::{CallToolResult, ContentBlock};
use serde::Serialize;
use serde_json::{Map, Value};

use crate::client::BunnyError;

pub type ToolResult = Result<CallToolResult, ErrorData>;

/// Success with one compact JSON text block (REQ-OUT-01, C2).
pub fn ok_json<T: Serialize>(v: &T) -> ToolResult {
    let text = serde_json::to_string(v).map_err(|e| {
        ErrorData::internal_error(format!("failed to serialize tool output: {e}"), None)
    })?;
    Ok(CallToolResult::success(vec![ContentBlock::text(text)]))
}

/// `{"ok":true, ...ctx}` for operations without a response payload (REQ-OUT-02).
/// `ctx` must be an object (or `null` for no context).
pub fn ok_unit(ctx: Value) -> ToolResult {
    let mut map = match ctx {
        Value::Object(map) => map,
        Value::Null => Map::new(),
        other => {
            return Err(ErrorData::internal_error(
                format!("ok_unit context must be an object, got {other}"),
                None,
            ));
        }
    };
    map.remove("ok");
    // serde_json maps are sorted (no `preserve_order`), so splice `"ok":true` in first by hand.
    let rest = serde_json::to_string(&map).map_err(|e| {
        ErrorData::internal_error(format!("failed to serialize tool output: {e}"), None)
    })?;
    let text = match rest.strip_prefix('{') {
        Some(fields) if !map.is_empty() => format!("{{\"ok\":true,{fields}"),
        _ => r#"{"ok":true}"#.to_string(),
    };
    Ok(CallToolResult::success(vec![ContentBlock::text(text)]))
}

/// Tool-level error (`is_error = true`), never a protocol error (C4).
pub fn tool_error(msg: impl Into<String>) -> ToolResult {
    Ok(CallToolResult::error(vec![ContentBlock::text(msg.into())]))
}

/// Maps a client error to its actionable text (design §4).
pub fn api_error(e: BunnyError) -> ToolResult {
    tool_error(e.to_tool_message())
}

/// Local argument validation failure: `Invalid argument: <msg>`.
pub fn invalid(msg: impl Into<String>) -> ToolResult {
    api_error(BunnyError::InvalidArgument(msg.into()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn text(r: &CallToolResult) -> &str {
        r.content[0].as_text().map(|t| t.text.as_str()).unwrap()
    }

    #[test]
    fn ok_json_is_compact_success() {
        let r = ok_json(&json!({"a": 1, "b": [1, 2]})).unwrap();
        assert_eq!(r.is_error, Some(false));
        assert_eq!(r.content.len(), 1);
        assert_eq!(text(&r), r#"{"a":1,"b":[1,2]}"#);
    }

    #[test]
    fn ok_unit_merges_context() {
        assert_eq!(
            text(&ok_unit(json!({"id": 7})).unwrap()),
            r#"{"ok":true,"id":7}"#
        );
        assert_eq!(
            text(&ok_unit(json!({"b": 1, "a": [2], "ok": false})).unwrap()),
            r#"{"ok":true,"a":[2],"b":1}"#
        );
        assert_eq!(text(&ok_unit(Value::Null).unwrap()), r#"{"ok":true}"#);
        assert!(ok_unit(json!(1)).is_err());
    }

    #[test]
    fn errors_are_tool_errors() {
        let r = invalid("pull_zone_id must be >= 1").unwrap();
        assert_eq!(r.is_error, Some(true));
        assert_eq!(text(&r), "Invalid argument: pull_zone_id must be >= 1");
        let r = api_error(BunnyError::RateLimited).unwrap();
        assert_eq!(r.is_error, Some(true));
        assert!(text(&r).contains("429"));
    }
}
