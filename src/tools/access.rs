//! Access list tools (`access_router`, specs/tools/access.md, REQ-PZ-08).
//!
//! Adds or removes entries in allowed referrers, blocked referrers, or blocked IPs.

use std::net::IpAddr;

use rmcp::handler::server::wrapper::Parameters;
use rmcp::{tool, tool_router};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::server::BunnyServer;
use crate::tools::util::{ToolResult, api_error, invalid, ok_unit};

/// Target list for pull zone access control.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AccessList {
    /// Whitelist of allowed HTTP referrers.
    AllowedReferrer,
    /// Blacklist of blocked HTTP referrers.
    BlockedReferrer,
    /// Blacklist of blocked IP addresses or CIDR ranges.
    BlockedIp,
}

/// Action to perform on an access list.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AccessAction {
    /// Add entry to list.
    Add,
    /// Remove entry from list.
    Remove,
}

/// Arguments for `bunny_pullzone_access_list`.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PullZoneAccessListArgs {
    /// Pull zone id.
    pub id: u64,
    /// Which list.
    pub list: AccessList,
    /// Operation.
    pub action: AccessAction,
    /// Hostname (referrers) or IP/CIDR (blocked_ip).
    pub value: String,
}

/// Validates that an IP address or CIDR range is well-formed.
///
/// Accepts standard IPv4/IPv6 addresses, or `ip/prefix` where prefix is within range
/// (0..=32 for IPv4, 0..=128 for IPv6). Returns normalized string.
pub fn validate_blocked_ip(input: &str) -> Result<String, String> {
    let s = input.trim();
    if s.is_empty() {
        return Err("value cannot be empty".to_string());
    }

    if let Ok(ip) = s.parse::<IpAddr>() {
        return Ok(ip.to_string());
    }

    if let Some((ip_str, prefix_str)) = s.split_once('/') {
        let ip = ip_str
            .parse::<IpAddr>()
            .map_err(|_| format!("invalid IP address in CIDR: '{ip_str}'"))?;
        let prefix = prefix_str
            .parse::<u8>()
            .map_err(|_| format!("invalid prefix in CIDR: '{prefix_str}'"))?;

        let max_prefix = match ip {
            IpAddr::V4(_) => 32,
            IpAddr::V6(_) => 128,
        };

        if prefix > max_prefix {
            return Err(format!(
                "prefix /{prefix} exceeds maximum of /{max_prefix} for {}",
                if ip.is_ipv4() { "IPv4" } else { "IPv6" }
            ));
        }

        return Ok(format!("{ip}/{prefix}"));
    }

    Err(format!("'{s}' is not a valid IP address or CIDR"))
}

/// Normalizes and validates a referrer hostname according to specs/tools/access.md.
///
/// If a scheme (`http://` or `https://` or other) is present, the host part is extracted.
/// If no scheme is present, paths, query strings, fragments, and ports are rejected.
/// The resulting host is returned in lowercase.
pub fn normalize_referrer(input: &str) -> Result<String, String> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err("value cannot be empty".to_string());
    }

    let host_part = if let Some((_scheme, rest)) = trimmed.split_once("://") {
        let authority = rest.split(['/', '?', '#']).next().unwrap_or("");
        let authority = if let Some((_userinfo, host_port)) = authority.split_once('@') {
            host_port
        } else {
            authority
        };
        authority.split(':').next().unwrap_or("")
    } else {
        if trimmed.contains('/') {
            return Err("referrer must not contain a path".to_string());
        }
        if trimmed.contains('?') || trimmed.contains('#') {
            return Err("referrer must not contain query parameters or fragments".to_string());
        }
        if trimmed.contains(':') {
            return Err("referrer must not contain a port".to_string());
        }
        trimmed
    };

    let host_part = host_part.trim();
    if host_part.is_empty() {
        return Err("referrer hostname cannot be empty".to_string());
    }

    Ok(host_part.to_ascii_lowercase())
}

#[tool_router(router = access_router, vis = "pub(crate)")]
impl BunnyServer {
    /// Add or remove an entry in a pull zone's allowed referrers, blocked referrers or blocked IPs.
    #[tool(
        name = "bunny_pullzone_access_list",
        annotations(
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = true
        )
    )]
    pub async fn bunny_pullzone_access_list(
        &self,
        Parameters(args): Parameters<PullZoneAccessListArgs>,
    ) -> ToolResult {
        if args.id == 0 {
            return invalid("id must be >= 1");
        }

        let value = match args.list {
            AccessList::BlockedIp => match validate_blocked_ip(&args.value) {
                Ok(v) => v,
                Err(e) => return invalid(e),
            },
            AccessList::AllowedReferrer | AccessList::BlockedReferrer => {
                match normalize_referrer(&args.value) {
                    Ok(v) => v,
                    Err(e) => return invalid(e),
                }
            }
        };

        let (path, body) = match (args.list, args.action) {
            (AccessList::AllowedReferrer, AccessAction::Add) => (
                format!("/pullzone/{}/addAllowedReferrer", args.id),
                json!({ "Hostname": value }),
            ),
            (AccessList::AllowedReferrer, AccessAction::Remove) => (
                format!("/pullzone/{}/removeAllowedReferrer", args.id),
                json!({ "Hostname": value }),
            ),
            (AccessList::BlockedReferrer, AccessAction::Add) => (
                format!("/pullzone/{}/addBlockedReferrer", args.id),
                json!({ "Hostname": value }),
            ),
            (AccessList::BlockedReferrer, AccessAction::Remove) => (
                format!("/pullzone/{}/removeBlockedReferrer", args.id),
                json!({ "Hostname": value }),
            ),
            (AccessList::BlockedIp, AccessAction::Add) => (
                format!("/pullzone/{}/addBlockedIp", args.id),
                json!({ "BlockedIp": value }),
            ),
            (AccessList::BlockedIp, AccessAction::Remove) => (
                format!("/pullzone/{}/removeBlockedIp", args.id),
                json!({ "BlockedIp": value }),
            ),
        };

        match self.client.post::<Value>(&path, &[], Some(&body)).await {
            Ok(_) => ok_unit(json!({
                "id": args.id,
                "list": args.list,
                "action": args.action,
                "value": value,
            })),
            Err(e) => api_error(e),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_blocked_ip() {
        assert_eq!(validate_blocked_ip("1.2.3.4").unwrap(), "1.2.3.4");
        assert_eq!(validate_blocked_ip("  1.2.3.4  ").unwrap(), "1.2.3.4");
        assert_eq!(validate_blocked_ip("::1").unwrap(), "::1");
        assert_eq!(validate_blocked_ip("2001:db8::1").unwrap(), "2001:db8::1");
        assert_eq!(
            validate_blocked_ip("192.168.1.0/24").unwrap(),
            "192.168.1.0/24"
        );
        assert_eq!(validate_blocked_ip("0.0.0.0/0").unwrap(), "0.0.0.0/0");
        assert_eq!(
            validate_blocked_ip("2001:db8::/32").unwrap(),
            "2001:db8::/32"
        );
        assert_eq!(validate_blocked_ip("::/0").unwrap(), "::/0");

        assert!(validate_blocked_ip("").is_err());
        assert!(validate_blocked_ip("   ").is_err());
        assert!(validate_blocked_ip("not-an-ip").is_err());
        assert!(validate_blocked_ip("999.999.999.999").is_err());
        assert!(validate_blocked_ip("1.2.3.4/33").is_err());
        assert!(validate_blocked_ip("2001:db8::/129").is_err());
        assert!(validate_blocked_ip("1.2.3.4/").is_err());
        assert!(validate_blocked_ip("/24").is_err());
        assert!(validate_blocked_ip("1.2.3.4/abc").is_err());
    }

    #[test]
    fn test_normalize_referrer() {
        assert_eq!(normalize_referrer("example.com").unwrap(), "example.com");
        assert_eq!(
            normalize_referrer("  EXAMPLE.COM  ").unwrap(),
            "example.com"
        );
        assert_eq!(
            normalize_referrer("https://example.com").unwrap(),
            "example.com"
        );
        assert_eq!(
            normalize_referrer("http://example.com").unwrap(),
            "example.com"
        );
        assert_eq!(
            normalize_referrer("https://example.com/").unwrap(),
            "example.com"
        );
        assert_eq!(
            normalize_referrer("https://example.com/path/to/page").unwrap(),
            "example.com"
        );
        assert_eq!(
            normalize_referrer("http://user:pass@example.com:8080/path").unwrap(),
            "example.com"
        );
        assert_eq!(
            normalize_referrer("*.example.com").unwrap(),
            "*.example.com"
        );
        assert_eq!(
            normalize_referrer("https://*.example.com").unwrap(),
            "*.example.com"
        );

        assert!(normalize_referrer("").is_err());
        assert!(normalize_referrer("   ").is_err());
        assert!(normalize_referrer("example.com/path").is_err());
        assert!(normalize_referrer("example.com:8080").is_err());
        assert!(normalize_referrer("example.com?query=1").is_err());
        assert!(normalize_referrer("https://").is_err());
        assert!(normalize_referrer("http:///").is_err());
    }
}
