//! Statistics domain tools (`stats_router`, design §6, specs/tools/stats.md, REQ-ST-01..03).

use rmcp::handler::server::wrapper::Parameters;
use rmcp::schemars::JsonSchema;
use rmcp::{tool, tool_router};
use serde::Deserialize;

use crate::model::stats::{
    DateRange, FeatureKind, PullzoneFeatureStatisticsResponse, StatisticsApiModel,
    StatisticsSummary, resolve_date_range,
};
use crate::server::BunnyServer;
use crate::tools::util::{ToolResult, api_error, invalid, ok_json};

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct StatisticsArgs {
    /// Limit to one pull zone.
    #[serde(default)]
    pub pullzone_id: Option<u64>,
    /// Start date (UTC, YYYY-MM-DD).
    #[serde(default)]
    pub date_from: Option<String>,
    /// End date.
    #[serde(default)]
    pub date_to: Option<String>,
    /// Hourly granularity (short ranges).
    #[serde(default)]
    pub hourly: Option<bool>,
    /// Include time series (≤60 points each).
    #[serde(default)]
    pub include_series: Option<bool>,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PullzoneFeatureStatisticsArgs {
    /// Pull zone id.
    pub pullzone_id: u64,
    /// Feature.
    pub kind: FeatureKind,
    /// Start date (UTC, YYYY-MM-DD).
    #[serde(default)]
    pub date_from: Option<String>,
    /// End date.
    #[serde(default)]
    pub date_to: Option<String>,
    /// Hourly granularity (short ranges).
    #[serde(default)]
    pub hourly: Option<bool>,
    /// Include time series (≤60 points each).
    #[serde(default)]
    pub include_series: Option<bool>,
}

#[tool_router(router = stats_router, vis = "pub(crate)")]
impl BunnyServer {
    /// Traffic statistics for the account or one pull zone: bandwidth, requests, cache hit rate, origin time, error counts and top countries.
    #[tool(
        name = "bunny_statistics",
        annotations(
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = false,
            open_world_hint = true
        )
    )]
    pub async fn bunny_statistics(
        &self,
        Parameters(args): Parameters<StatisticsArgs>,
    ) -> ToolResult {
        if let Some(id) = args.pullzone_id
            && id == 0
        {
            return invalid("pullzone_id must be >= 1");
        }

        let (from, to) =
            match resolve_date_range(args.date_from.as_deref(), args.date_to.as_deref()) {
                Ok(range) => range,
                Err(err) => return invalid(err),
            };

        let hourly = args.hourly.unwrap_or(false);
        let include_series = args.include_series.unwrap_or(false);

        let mut query: Vec<(&str, String)> = Vec::new();
        query.push(("dateFrom", from.to_rfc3339_start()));
        query.push(("dateTo", to.to_rfc3339_end()));
        if let Some(id) = args.pullzone_id {
            query.push(("pullZone", id.to_string()));
        }
        query.push(("hourly", hourly.to_string()));
        query.push(("loadErrors", "true".to_string()));
        query.push(("loadOriginResponseTimes", "true".to_string()));
        query.push(("loadGeographicTrafficDistribution", "true".to_string()));

        match self
            .client
            .get::<StatisticsApiModel>("/statistics", &query)
            .await
        {
            Ok(api) => {
                let range = DateRange {
                    from: from.to_iso_date(),
                    to: to.to_iso_date(),
                };
                let summary =
                    StatisticsSummary::from_api(api, args.pullzone_id, range, include_series);
                ok_json(&summary)
            }
            Err(e) => api_error(e),
        }
    }

    /// Statistics for a pull zone feature: optimizer, origin shield queue or SafeHop.
    #[tool(
        name = "bunny_pullzone_feature_statistics",
        annotations(
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = false,
            open_world_hint = true
        )
    )]
    pub async fn bunny_pullzone_feature_statistics(
        &self,
        Parameters(args): Parameters<PullzoneFeatureStatisticsArgs>,
    ) -> ToolResult {
        if args.pullzone_id == 0 {
            return invalid("pullzone_id must be >= 1");
        }

        let (from, to) =
            match resolve_date_range(args.date_from.as_deref(), args.date_to.as_deref()) {
                Ok(range) => range,
                Err(err) => return invalid(err),
            };

        let hourly = args.hourly.unwrap_or(false);
        let include_series = args.include_series.unwrap_or(false);

        let path = match args.kind {
            FeatureKind::Optimizer => {
                format!("/pullzone/{}/optimizer/statistics", args.pullzone_id)
            }
            FeatureKind::OriginShieldQueue => {
                format!(
                    "/pullzone/{}/originshield/queuestatistics",
                    args.pullzone_id
                )
            }
            FeatureKind::Safehop => {
                format!("/pullzone/{}/safehop/statistics", args.pullzone_id)
            }
        };

        let query: Vec<(&str, String)> = vec![
            ("dateFrom", from.to_rfc3339_start()),
            ("dateTo", to.to_rfc3339_end()),
            ("hourly", hourly.to_string()),
        ];

        match self.client.get::<serde_json::Value>(&path, &query).await {
            Ok(raw) => {
                let range = DateRange {
                    from: from.to_iso_date(),
                    to: to.to_iso_date(),
                };
                match PullzoneFeatureStatisticsResponse::from_value(
                    &raw,
                    args.pullzone_id,
                    args.kind,
                    range,
                    include_series,
                ) {
                    Ok(resp) => ok_json(&resp),
                    Err(e) => api_error(e),
                }
            }
            Err(e) => api_error(e),
        }
    }
}
