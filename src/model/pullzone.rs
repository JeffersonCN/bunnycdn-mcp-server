//! Pull zone models and projections (design §8, specs/tools/pullzone.md).
//!
//! Tolerant models for deserializing Bunny API responses and projections for MCP output.

use rmcp::schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::client::BunnyError;

/// Compact representation of a pull zone in listings (TOOL-PZ-01).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct PullZoneListItem {
    pub id: u64,
    pub name: String,
    pub enabled: bool,
    pub tier: String,
    pub origin: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub hostnames: Vec<String>,
    pub monthly_bandwidth_used: i64,
}

impl PullZoneListItem {
    pub fn from_value(v: &serde_json::Value) -> Result<Self, BunnyError> {
        let raw: RawPullZone = serde_json::from_value(v.clone()).map_err(|e| {
            BunnyError::Decode(format!("failed to decode pull zone list item: {e}"))
        })?;
        Ok(raw.to_list_item())
    }
}

/// Paginated pull zone list response for MCP output (TOOL-PZ-01).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct PullZoneListResponse {
    pub items: Vec<PullZoneListItem>,
    pub page: u32,
    pub per_page: u32,
    pub total: u32,
    pub has_more: bool,
}

/// Full operational summary of a pull zone (TOOL-PZ-02, T11, T91).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct PullZoneSummary {
    pub id: u64,
    pub name: String,
    pub enabled: bool,
    pub suspended: bool,
    pub tier: String,
    pub origin: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub origin_host_header: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub hostnames: Vec<PullZoneHostnameSummary>,
    pub edge_rules: usize,
    pub cache: PullZoneCacheSummary,
    pub security: PullZoneSecuritySummary,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub features: Vec<String>,
    pub usage: PullZoneUsageSummary,
}

impl PullZoneSummary {
    pub fn from_value(v: &serde_json::Value) -> Result<Self, BunnyError> {
        let raw: RawPullZone = serde_json::from_value(v.clone())
            .map_err(|e| BunnyError::Decode(format!("failed to decode pull zone summary: {e}")))?;
        Ok(raw.to_summary())
    }
}

/// Hostname entry within a pull zone summary.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct PullZoneHostnameSummary {
    pub value: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub force_ssl: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub has_certificate: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system: Option<bool>,
}

/// Cache settings within a pull zone summary.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct PullZoneCacheSummary {
    pub smart_cache: bool,
    pub max_age_override: i64,
    pub ignore_query_strings: bool,
    pub stale_while_updating: bool,
    pub stale_while_offline: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub origin_shield: Option<String>,
}

/// Security settings within a pull zone summary.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct PullZoneSecuritySummary {
    pub token_auth: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub allowed_referrers: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub blocked_referrers: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub blocked_ips: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub blocked_countries: Vec<String>,
}

/// Usage statistics within a pull zone summary.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct PullZoneUsageSummary {
    pub monthly_bandwidth_used: i64,
    pub monthly_charges: f64,
}

/// Tolerant raw model matching Bunny API `PullZoneModel`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct RawPullZone {
    pub id: Option<u64>,
    pub name: Option<String>,
    pub origin_url: Option<String>,
    pub enabled: Option<bool>,
    pub suspended: Option<bool>,
    #[serde(default)]
    pub hostnames: Vec<RawHostname>,
    pub storage_zone_id: Option<u64>,
    pub edge_script_id: Option<u64>,
    pub middleware_script_id: Option<u64>,
    pub magic_containers_app_id: Option<String>,
    #[serde(default)]
    pub allowed_referrers: Vec<String>,
    #[serde(default)]
    pub blocked_referrers: Vec<String>,
    #[serde(default)]
    pub blocked_ips: Vec<String>,
    #[serde(default)]
    pub blocked_countries: Vec<String>,
    pub zone_security_enabled: Option<bool>,
    pub zone_security_key: Option<String>,
    pub ignore_query_strings: Option<bool>,
    pub monthly_bandwidth_used: Option<i64>,
    pub monthly_charges: Option<f64>,
    pub origin_host_header: Option<String>,
    #[serde(default, deserialize_with = "crate::model::int_or_string")]
    pub r#type: Option<i64>,
    #[serde(default, deserialize_with = "crate::model::int_or_string")]
    pub origin_type: Option<i64>,
    pub enable_origin_shield: Option<bool>,
    pub origin_shield_zone_code: Option<String>,
    pub cache_control_max_age_override: Option<i64>,
    pub enable_logging: Option<bool>,
    pub enable_smart_cache: Option<bool>,
    #[serde(default)]
    pub edge_rules: Vec<serde_json::Value>,
    pub optimizer_enabled: Option<bool>,
    pub use_stale_while_updating: Option<bool>,
    pub use_stale_while_offline: Option<bool>,
    pub enable_safe_hop: Option<bool>,
    #[serde(rename = "EnableAutoSSL", alias = "EnableAutoSsl")]
    pub enable_auto_ssl: Option<bool>,
    #[serde(
        rename = "ShieldDDosProtectionEnabled",
        alias = "ShieldDdosProtectionEnabled"
    )]
    pub shield_ddos_protection_enabled: Option<bool>,
    pub enable_request_coalescing: Option<bool>,
    pub enable_bunny_image_ai: Option<bool>,
    pub load_balancer_id: Option<u64>,
    pub enable_web_sockets: Option<bool>,
}

/// Tolerant raw model matching Bunny API `HostnameModel`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct RawHostname {
    pub id: Option<u64>,
    pub value: Option<String>,
    #[serde(rename = "ForceSSL", alias = "ForceSsl")]
    pub force_ssl: Option<bool>,
    pub is_system_hostname: Option<bool>,
    pub has_certificate: Option<bool>,
}

/// Untagged API response for `GET /pullzone`: either a JSON array or a paginated object.
#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum PullZoneListApiResponse {
    Paginated(RawPaginatedPullZones),
    Array(Vec<RawPullZone>),
}

impl PullZoneListApiResponse {
    pub fn into_response(
        self,
        requested_page: u32,
        requested_per_page: u32,
    ) -> PullZoneListResponse {
        match self {
            Self::Array(list) => {
                let total = list.len() as u32;
                let start = ((requested_page - 1).saturating_mul(requested_per_page)) as usize;
                let items: Vec<PullZoneListItem> = if start < list.len() {
                    list.into_iter()
                        .skip(start)
                        .take(requested_per_page as usize)
                        .map(|z| z.to_list_item())
                        .collect()
                } else {
                    Vec::new()
                };
                let has_more = (start + items.len()) < (total as usize);
                PullZoneListResponse {
                    items,
                    page: requested_page,
                    per_page: requested_per_page,
                    total,
                    has_more,
                }
            }
            Self::Paginated(p) => {
                let items: Vec<PullZoneListItem> =
                    p.items.into_iter().map(|z| z.to_list_item()).collect();
                let total = p.total_items.unwrap_or(items.len() as u32);
                let has_more = p.has_more_items.unwrap_or(false);
                PullZoneListResponse {
                    items,
                    page: p.current_page.unwrap_or(requested_page),
                    per_page: requested_per_page,
                    total,
                    has_more,
                }
            }
        }
    }
}

/// Paginated API response structure `{Items, CurrentPage, TotalItems, HasMoreItems}`.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct RawPaginatedPullZones {
    #[serde(default)]
    pub items: Vec<RawPullZone>,
    pub current_page: Option<u32>,
    pub total_items: Option<u32>,
    pub has_more_items: Option<bool>,
}

/// Derives `origin` string representation from PullZone fields according to contract.
pub fn compute_origin(
    origin_type: Option<i64>,
    origin_url: Option<&str>,
    storage_zone_id: Option<u64>,
    edge_script_id: Option<u64>,
    load_balancer_id: Option<u64>,
    magic_containers_app_id: Option<&str>,
) -> String {
    match origin_type {
        Some(2) => format!("storage:{}", storage_zone_id.unwrap_or(0)),
        Some(4) => {
            if let Some(id) = edge_script_id.filter(|&id| id > 0) {
                format!("edge_script:{id}")
            } else {
                "edge_script".to_string()
            }
        }
        Some(3) => {
            if let Some(id) = load_balancer_id.filter(|&id| id > 0) {
                format!("load_balancer:{id}")
            } else {
                "load_balancer".to_string()
            }
        }
        Some(5) => {
            if let Some(app_id) = magic_containers_app_id.filter(|s| !s.is_empty()) {
                format!("magic_containers:{app_id}")
            } else {
                "magic_containers".to_string()
            }
        }
        Some(1) => "dns_accelerate".to_string(),
        Some(6) => "push_zone".to_string(),
        _ => origin_url.unwrap_or("").to_string(),
    }
}

/// Derives `tier` string ("standard" or "volume") from `Type`.
pub fn compute_tier(r#type: Option<i64>) -> String {
    match r#type {
        Some(1) => "volume".to_string(),
        _ => "standard".to_string(),
    }
}

/// Builds active feature names list.
pub fn compute_features(raw: &RawPullZone) -> Vec<String> {
    let mut features = Vec::new();
    if raw.optimizer_enabled.unwrap_or(false) {
        features.push("optimizer".to_string());
    }
    if raw.enable_logging.unwrap_or(false) {
        features.push("logging".to_string());
    }
    if raw.enable_web_sockets.unwrap_or(false) {
        features.push("websockets".to_string());
    }
    if raw.enable_safe_hop.unwrap_or(false) {
        features.push("safehop".to_string());
    }
    if raw.enable_request_coalescing.unwrap_or(false) {
        features.push("request_coalescing".to_string());
    }
    if raw.enable_auto_ssl.unwrap_or(false) {
        features.push("auto_ssl".to_string());
    }
    if raw.enable_bunny_image_ai.unwrap_or(false) {
        features.push("image_ai".to_string());
    }
    if raw.shield_ddos_protection_enabled.unwrap_or(false) {
        features.push("ddos_protection".to_string());
    }
    features
}

impl RawPullZone {
    pub fn to_summary(&self) -> PullZoneSummary {
        let origin = compute_origin(
            self.origin_type,
            self.origin_url.as_deref(),
            self.storage_zone_id,
            self.edge_script_id,
            self.load_balancer_id,
            self.magic_containers_app_id.as_deref(),
        );
        let tier = compute_tier(self.r#type);
        let hostnames = self
            .hostnames
            .iter()
            .map(|h| PullZoneHostnameSummary {
                value: h.value.clone().unwrap_or_default(),
                force_ssl: if h.force_ssl.unwrap_or(false) {
                    Some(true)
                } else {
                    None
                },
                has_certificate: if h.has_certificate.unwrap_or(false) {
                    Some(true)
                } else {
                    None
                },
                system: if h.is_system_hostname.unwrap_or(false) {
                    Some(true)
                } else {
                    None
                },
            })
            .collect();

        let origin_shield = if self.enable_origin_shield.unwrap_or(false) {
            self.origin_shield_zone_code
                .clone()
                .filter(|s| !s.is_empty())
        } else {
            None
        };

        PullZoneSummary {
            id: self.id.unwrap_or(0),
            name: self.name.clone().unwrap_or_default(),
            enabled: self.enabled.unwrap_or(false),
            suspended: self.suspended.unwrap_or(false),
            tier,
            origin,
            origin_host_header: self.origin_host_header.clone().filter(|s| !s.is_empty()),
            hostnames,
            edge_rules: self.edge_rules.len(),
            cache: PullZoneCacheSummary {
                smart_cache: self.enable_smart_cache.unwrap_or(false),
                max_age_override: self.cache_control_max_age_override.unwrap_or(0),
                ignore_query_strings: self.ignore_query_strings.unwrap_or(false),
                stale_while_updating: self.use_stale_while_updating.unwrap_or(false),
                stale_while_offline: self.use_stale_while_offline.unwrap_or(false),
                origin_shield,
            },
            security: PullZoneSecuritySummary {
                token_auth: self.zone_security_enabled.unwrap_or(false),
                allowed_referrers: self.allowed_referrers.clone(),
                blocked_referrers: self.blocked_referrers.clone(),
                blocked_ips: self.blocked_ips.clone(),
                blocked_countries: self.blocked_countries.clone(),
            },
            features: compute_features(self),
            usage: PullZoneUsageSummary {
                monthly_bandwidth_used: self.monthly_bandwidth_used.unwrap_or(0),
                monthly_charges: self.monthly_charges.unwrap_or(0.0),
            },
        }
    }

    pub fn to_list_item(&self) -> PullZoneListItem {
        let origin = compute_origin(
            self.origin_type,
            self.origin_url.as_deref(),
            self.storage_zone_id,
            self.edge_script_id,
            self.load_balancer_id,
            self.magic_containers_app_id.as_deref(),
        );
        let tier = compute_tier(self.r#type);
        let hostnames = self
            .hostnames
            .iter()
            .filter_map(|h| h.value.clone())
            .collect();

        PullZoneListItem {
            id: self.id.unwrap_or(0),
            name: self.name.clone().unwrap_or_default(),
            enabled: self.enabled.unwrap_or(false),
            tier,
            origin,
            hostnames,
            monthly_bandwidth_used: self.monthly_bandwidth_used.unwrap_or(0),
        }
    }
}
