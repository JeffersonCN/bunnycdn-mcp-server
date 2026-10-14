//! Pull zone domain tools (design §6, specs/tools/pullzone.md).
//!
//! Implements read tools for pull zones: list, get (summary/verbose), and check availability.

use rmcp::handler::server::wrapper::Parameters;
use rmcp::schemars::JsonSchema;
use rmcp::{tool, tool_router};
use serde::Deserialize;

use crate::client::BunnyError;
use crate::model::pullzone::{PullZoneListApiResponse, PullZoneSummary};
use crate::server::BunnyServer;
use crate::tools::util::{ToolResult, api_error, invalid, ok_json};

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

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct CheckAvailabilityResponse {
    available: Option<bool>,
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
        match self.client.get::<serde_json::Value>(&path, &[]).await {
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

        let body = serde_json::json!({ "Name": args.name });
        match self
            .client
            .post::<serde_json::Value>("/pullzone/checkavailability", &[], Some(&body))
            .await
        {
            Ok(val) => {
                let resp: Result<CheckAvailabilityResponse, _> = serde_json::from_value(val);
                match resp {
                    Ok(CheckAvailabilityResponse {
                        available: Some(available),
                    }) => ok_json(&serde_json::json!({
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
}
