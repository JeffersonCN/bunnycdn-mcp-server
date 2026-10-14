//! Reference tools: regions and countries (`reference_router`, design §6, REQ-REF-01, REQ-REF-02).

use rmcp::handler::server::wrapper::Parameters;
use rmcp::{tool, tool_router};
use schemars::JsonSchema;
use serde::Deserialize;

use crate::model::reference::{
    CountryApiModel, CountryListResponse, CountrySummary, RegionApiModel, RegionListResponse,
    RegionSummary,
};
use crate::server::BunnyServer;
use crate::tools::util::{ToolResult, api_error, ok_json};

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RegionListArgs {
    /// Filter by continent code (e.g. "EU"), case-insensitive.
    pub continent: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CountryListArgs {
    /// Search filter matching country name or ISO code, case-insensitive.
    pub search: Option<String>,
}

#[tool_router(router = reference_router, vis = "pub(crate)")]
impl BunnyServer {
    /// List Bunny CDN regions (PoPs) with region code and price per GB.
    #[tool(
        name = "bunny_region_list",
        annotations(
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = true
        )
    )]
    pub async fn bunny_region_list(
        &self,
        Parameters(args): Parameters<RegionListArgs>,
    ) -> ToolResult {
        let regions = match self
            .client
            .get::<Option<Vec<RegionApiModel>>>("/region", &[])
            .await
        {
            Ok(list) => {
                let mut projected: Vec<RegionSummary> = list
                    .unwrap_or_default()
                    .into_iter()
                    .map(RegionSummary::from)
                    .collect();

                if let Some(continent_filter) = &args.continent {
                    let filter = continent_filter.trim();
                    if !filter.is_empty() {
                        projected.retain(|r| {
                            r.continent
                                .as_deref()
                                .is_some_and(|c| c.eq_ignore_ascii_case(filter))
                        });
                    }
                }
                projected
            }
            Err(e) => return api_error(e),
        };

        ok_json(&RegionListResponse { regions })
    }

    /// List countries with ISO codes (useful for blocked_countries and geo statistics).
    #[tool(
        name = "bunny_country_list",
        annotations(
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = true
        )
    )]
    pub async fn bunny_country_list(
        &self,
        Parameters(args): Parameters<CountryListArgs>,
    ) -> ToolResult {
        let countries = match self
            .client
            .get::<Option<Vec<CountryApiModel>>>("/country", &[])
            .await
        {
            Ok(list) => {
                let mut projected: Vec<CountrySummary> = list
                    .unwrap_or_default()
                    .into_iter()
                    .map(CountrySummary::from)
                    .collect();

                if let Some(search_filter) = &args.search {
                    let filter = search_filter.trim().to_lowercase();
                    if !filter.is_empty() {
                        projected.retain(|c| {
                            c.name
                                .as_deref()
                                .is_some_and(|n| n.to_lowercase().contains(&filter))
                                || c.iso
                                    .as_deref()
                                    .is_some_and(|i| i.to_lowercase().contains(&filter))
                        });
                    }
                }
                projected
            }
            Err(e) => return api_error(e),
        };

        ok_json(&CountryListResponse { countries })
    }
}
