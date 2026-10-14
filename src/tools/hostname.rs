//! Hostname and SSL certificate tools (specs/tools/hostname.md, REQ-HS-01..05).

use base64::Engine;
use base64::prelude::BASE64_STANDARD;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::{tool, tool_router};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};

use crate::client::BunnyError;
use crate::server::BunnyServer;
use crate::tools::util::{ToolResult, api_error, invalid, ok_unit, tool_error};

/// Pure function validating and normalizing a hostname.
///
/// Rules:
/// - Trim whitespace.
/// - If prefixed with `https://` (case-insensitive), extracts the host portion.
/// - Without `https://`, must not contain a scheme, path, query, fragment, or port.
/// - Lowercase ASCII.
/// - Total length <= 253 characters.
/// - Dot-separated labels of 1..=63 characters containing `[a-z0-9-]`, not starting or ending with `-`.
/// - Wildcard `*.` allowed only when `allow_wildcard` is true, and only as the first label followed by at least one domain label.
pub fn normalize_hostname(input: &str, allow_wildcard: bool) -> Result<String, String> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err("hostname cannot be empty".to_string());
    }

    let lower = trimmed.to_ascii_lowercase();

    // If starting with https://, extract host part from authority (before '/', '?', '#', ':')
    let host_part = if let Some(rest) = lower.strip_prefix("https://") {
        let authority = rest.split(['/', '?', '#']).next().unwrap_or("");
        if authority.contains('@') {
            return Err("hostname must not contain userinfo".to_string());
        }
        let host = authority.split(':').next().unwrap_or("");
        if host.is_empty() {
            return Err("hostname cannot be empty".to_string());
        }
        host
    } else {
        if lower.contains("://") {
            return Err("hostname must not contain a scheme".to_string());
        }
        if lower.contains('/') {
            return Err("hostname must not contain a path".to_string());
        }
        if lower.contains('?') || lower.contains('#') {
            return Err("hostname must not contain query parameters or fragments".to_string());
        }
        if lower.contains(':') {
            return Err("hostname must not contain a port".to_string());
        }
        lower.as_str()
    };

    if host_part.contains('@') {
        return Err("hostname must not contain userinfo".to_string());
    }

    if host_part.len() > 253 {
        return Err("hostname exceeds maximum length of 253 characters".to_string());
    }

    let labels: Vec<&str> = host_part.split('.').collect();
    if labels.is_empty() {
        return Err("hostname cannot be empty".to_string());
    }

    for (i, label) in labels.iter().enumerate() {
        if label.is_empty() {
            return Err("hostname label cannot be empty".to_string());
        }
        if label.len() > 63 {
            return Err("hostname label exceeds maximum length of 63 characters".to_string());
        }

        if i == 0 && allow_wildcard && *label == "*" {
            if labels.len() < 2 {
                return Err("wildcard hostname must contain a domain after '*.'".to_string());
            }
            continue;
        }

        if !label
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
        {
            return Err(format!(
                "hostname label '{label}' contains invalid characters (allowed: [a-z0-9-])"
            ));
        }

        if label.starts_with('-') || label.ends_with('-') {
            return Err(format!(
                "hostname label '{label}' cannot start or end with a hyphen"
            ));
        }
    }

    Ok(host_part.to_string())
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct HostnameAddArgs {
    /// Pull zone id.
    pub id: u64,
    /// e.g. cdn.example.com
    pub hostname: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct HostnameRemoveArgs {
    /// Pull zone id.
    pub id: u64,
    /// Custom hostname to remove.
    pub hostname: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct HostnameForceSslArgs {
    /// Pull zone id.
    pub id: u64,
    /// Custom hostname to configure.
    pub hostname: String,
    /// Whether to force SSL redirection.
    pub enabled: bool,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CertificateIssueFreeArgs {
    /// Hostname already added to a pull zone with DNS pointing to Bunny.
    pub hostname: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CertificateAddArgs {
    /// Pull zone id.
    pub id: u64,
    /// Hostname the certificate is for.
    pub hostname: String,
    /// Certificate chain in PEM.
    pub certificate_pem: String,
    /// Private key in PEM.
    pub private_key_pem: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CertificateRemoveArgs {
    /// Pull zone id.
    pub id: u64,
    /// Hostname to remove certificate from.
    pub hostname: String,
}

#[tool_router(router = hostname_router, vis = "pub(crate)")]
impl BunnyServer {
    /// Add a custom hostname to a pull zone. Point its DNS (CNAME) to the zone's b-cdn.net hostname first.
    #[tool(
        name = "bunny_hostname_add",
        annotations(
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = false,
            open_world_hint = true
        )
    )]
    pub async fn bunny_hostname_add(
        &self,
        Parameters(args): Parameters<HostnameAddArgs>,
    ) -> ToolResult {
        if args.id == 0 {
            return invalid("id must be >= 1");
        }
        let hostname = match normalize_hostname(&args.hostname, false) {
            Ok(h) => h,
            Err(e) => return invalid(e),
        };

        let path = format!("/pullzone/{}/addHostname", args.id);
        let body = json!({ "Hostname": hostname });
        match self.client.post::<Value>(&path, &[], Some(&body)).await {
            Ok(_) => ok_unit(json!({
                "id": args.id,
                "hostname": hostname,
                "next": "run bunny_certificate_issue_free to enable HTTPS",
            })),
            Err(e) => api_error(e),
        }
    }

    /// Remove a custom hostname (and its certificate) from a pull zone.
    #[tool(
        name = "bunny_hostname_remove",
        annotations(
            read_only_hint = false,
            destructive_hint = true,
            idempotent_hint = true,
            open_world_hint = true
        )
    )]
    pub async fn bunny_hostname_remove(
        &self,
        Parameters(args): Parameters<HostnameRemoveArgs>,
    ) -> ToolResult {
        if args.id == 0 {
            return invalid("id must be >= 1");
        }
        let hostname = match normalize_hostname(&args.hostname, false) {
            Ok(h) => h,
            Err(e) => return invalid(e),
        };

        let path = format!("/pullzone/{}/removeHostname", args.id);
        let body = json!({ "Hostname": hostname });
        match self.client.delete::<Value>(&path, Some(&body)).await {
            Ok(_) => ok_unit(json!({
                "id": args.id,
                "hostname": hostname,
            })),
            Err(e) => api_error(e),
        }
    }

    /// Enable or disable HTTP→HTTPS redirect for one hostname.
    #[tool(
        name = "bunny_hostname_force_ssl",
        annotations(
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = true
        )
    )]
    pub async fn bunny_hostname_force_ssl(
        &self,
        Parameters(args): Parameters<HostnameForceSslArgs>,
    ) -> ToolResult {
        if args.id == 0 {
            return invalid("id must be >= 1");
        }
        let hostname = match normalize_hostname(&args.hostname, false) {
            Ok(h) => h,
            Err(e) => return invalid(e),
        };

        let path = format!("/pullzone/{}/setForceSSL", args.id);
        let body = json!({
            "Hostname": hostname,
            "ForceSSL": args.enabled,
        });
        match self.client.post::<Value>(&path, &[], Some(&body)).await {
            Ok(_) => ok_unit(json!({
                "id": args.id,
                "hostname": hostname,
                "force_ssl": args.enabled,
            })),
            Err(e) => api_error(e),
        }
    }

    /// Issue a free Let's Encrypt certificate for a hostname already added to a pull zone with DNS pointing to Bunny.
    #[tool(
        name = "bunny_certificate_issue_free",
        annotations(
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = true
        )
    )]
    pub async fn bunny_certificate_issue_free(
        &self,
        Parameters(args): Parameters<CertificateIssueFreeArgs>,
    ) -> ToolResult {
        let hostname = match normalize_hostname(&args.hostname, false) {
            Ok(h) => h,
            Err(e) => return invalid(e),
        };

        match self
            .client
            .get::<Value>(
                "/pullzone/loadFreeCertificate",
                &[("hostname", hostname.clone())],
            )
            .await
        {
            Ok(_) => ok_unit(json!({ "hostname": hostname })),
            Err(e) => {
                if matches!(e, BunnyError::Api { status: 400, .. }) {
                    let base = e.to_tool_message();
                    let hint = format!(
                        "Check that {hostname} has a CNAME to the zone's b-cdn.net hostname and retry in a few minutes."
                    );
                    let msg = if base.ends_with('.') {
                        format!("{base} {hint}")
                    } else {
                        format!("{base}. {hint}")
                    };
                    tool_error(msg)
                } else {
                    api_error(e)
                }
            }
        }
    }

    /// Upload a custom TLS certificate (PEM) for a hostname of a pull zone.
    #[tool(
        name = "bunny_certificate_add",
        annotations(
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = true
        )
    )]
    pub async fn bunny_certificate_add(
        &self,
        Parameters(args): Parameters<CertificateAddArgs>,
    ) -> ToolResult {
        if args.id == 0 {
            return invalid("id must be >= 1");
        }
        let hostname = match normalize_hostname(&args.hostname, true) {
            Ok(h) => h,
            Err(e) => return invalid(e),
        };

        if !args.certificate_pem.contains("-----BEGIN ") {
            return invalid("certificate_pem must be PEM-encoded (must contain '-----BEGIN ')");
        }
        if !args.private_key_pem.contains("-----BEGIN ") {
            return invalid("private_key_pem must be PEM-encoded (must contain '-----BEGIN ')");
        }

        let cert_b64 = BASE64_STANDARD.encode(args.certificate_pem.as_bytes());
        let key_b64 = BASE64_STANDARD.encode(args.private_key_pem.as_bytes());

        let path = format!("/pullzone/{}/addCertificate", args.id);
        let body = json!({
            "Hostname": hostname,
            "Certificate": cert_b64,
            "CertificateKey": key_b64,
        });

        match self.client.post::<Value>(&path, &[], Some(&body)).await {
            Ok(_) => ok_unit(json!({
                "id": args.id,
                "hostname": hostname,
            })),
            Err(e) => api_error(e),
        }
    }

    /// Remove the TLS certificate of a hostname.
    #[tool(
        name = "bunny_certificate_remove",
        annotations(
            read_only_hint = false,
            destructive_hint = true,
            idempotent_hint = true,
            open_world_hint = true
        )
    )]
    pub async fn bunny_certificate_remove(
        &self,
        Parameters(args): Parameters<CertificateRemoveArgs>,
    ) -> ToolResult {
        if args.id == 0 {
            return invalid("id must be >= 1");
        }
        let hostname = match normalize_hostname(&args.hostname, false) {
            Ok(h) => h,
            Err(e) => return invalid(e),
        };

        let path = format!("/pullzone/{}/removeCertificate", args.id);
        let body = json!({ "Hostname": hostname });
        match self.client.delete::<Value>(&path, Some(&body)).await {
            Ok(_) => ok_unit(json!({
                "id": args.id,
                "hostname": hostname,
            })),
            Err(e) => api_error(e),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn table_normalize_hostname() {
        struct Case {
            input: &'static str,
            allow_wildcard: bool,
            expected: Result<&'static str, &'static str>,
        }

        let cases = [
            // Valid standard hostnames
            Case {
                input: "cdn.example.com",
                allow_wildcard: false,
                expected: Ok("cdn.example.com"),
            },
            Case {
                input: "  CDN.EXAMPLE.COM  ",
                allow_wildcard: false,
                expected: Ok("cdn.example.com"),
            },
            Case {
                input: "https://cdn.example.com",
                allow_wildcard: false,
                expected: Ok("cdn.example.com"),
            },
            Case {
                input: "https://cdn.example.com/",
                allow_wildcard: false,
                expected: Ok("cdn.example.com"),
            },
            Case {
                input: "https://cdn.example.com/some/path?query=1#frag",
                allow_wildcard: false,
                expected: Ok("cdn.example.com"),
            },
            Case {
                input: "https://CDN.EXAMPLE.COM:443/",
                allow_wildcard: false,
                expected: Ok("cdn.example.com"),
            },
            Case {
                input: "sub-domain.example-site.com",
                allow_wildcard: false,
                expected: Ok("sub-domain.example-site.com"),
            },
            Case {
                input: "123.example.com",
                allow_wildcard: false,
                expected: Ok("123.example.com"),
            },
            // Wildcard cases
            Case {
                input: "*.example.com",
                allow_wildcard: true,
                expected: Ok("*.example.com"),
            },
            Case {
                input: "https://*.example.com",
                allow_wildcard: true,
                expected: Ok("*.example.com"),
            },
            Case {
                input: "*.sub.example.com",
                allow_wildcard: true,
                expected: Ok("*.sub.example.com"),
            },
            Case {
                input: "*.example.com",
                allow_wildcard: false,
                expected: Err("disallowed wildcard"),
            },
            Case {
                input: "*",
                allow_wildcard: true,
                expected: Err("wildcard domain required"),
            },
            Case {
                input: "*.*.example.com",
                allow_wildcard: true,
                expected: Err("invalid wildcard in later label"),
            },
            Case {
                input: "cdn.*.example.com",
                allow_wildcard: true,
                expected: Err("wildcard in middle label"),
            },
            Case {
                input: "*example.com",
                allow_wildcard: true,
                expected: Err("invalid characters"),
            },
            // Invalid schemes, paths, ports
            Case {
                input: "http://cdn.example.com",
                allow_wildcard: false,
                expected: Err("scheme"),
            },
            Case {
                input: "ftp://cdn.example.com",
                allow_wildcard: false,
                expected: Err("scheme"),
            },
            Case {
                input: "cdn.example.com:8080",
                allow_wildcard: false,
                expected: Err("port"),
            },
            Case {
                input: "cdn.example.com/path",
                allow_wildcard: false,
                expected: Err("path"),
            },
            Case {
                input: "cdn.example.com?query",
                allow_wildcard: false,
                expected: Err("query"),
            },
            Case {
                input: "cdn.example.com#fragment",
                allow_wildcard: false,
                expected: Err("fragment"),
            },
            Case {
                input: "https://user:pass@cdn.example.com",
                allow_wildcard: false,
                expected: Err("userinfo"),
            },
            // Empty / whitespace
            Case {
                input: "",
                allow_wildcard: false,
                expected: Err("empty"),
            },
            Case {
                input: "   ",
                allow_wildcard: false,
                expected: Err("empty"),
            },
            Case {
                input: "https://",
                allow_wildcard: false,
                expected: Err("empty"),
            },
            Case {
                input: "https:///",
                allow_wildcard: false,
                expected: Err("empty"),
            },
            // Label edge cases
            Case {
                input: "cdn..example.com",
                allow_wildcard: false,
                expected: Err("empty label"),
            },
            Case {
                input: ".example.com",
                allow_wildcard: false,
                expected: Err("empty label"),
            },
            Case {
                input: "example.com.",
                allow_wildcard: false,
                expected: Err("empty label"),
            },
            Case {
                input: "-cdn.example.com",
                allow_wildcard: false,
                expected: Err("starts with hyphen"),
            },
            Case {
                input: "cdn-.example.com",
                allow_wildcard: false,
                expected: Err("ends with hyphen"),
            },
            Case {
                input: "cdn.example_site.com",
                allow_wildcard: false,
                expected: Err("underscore not allowed"),
            },
            Case {
                input: "cdn.example.c@m",
                allow_wildcard: false,
                expected: Err("at sign not allowed"),
            },
        ];

        for (idx, c) in cases.iter().enumerate() {
            let res = normalize_hostname(c.input, c.allow_wildcard);
            match (res, c.expected) {
                (Ok(actual), Ok(exp)) => {
                    assert_eq!(actual, exp, "case {idx}: input '{}'", c.input);
                }
                (Err(err), Err(_)) => {
                    // expected failure
                    assert!(!err.is_empty(), "case {idx} returned empty error string");
                }
                (Ok(actual), Err(_)) => {
                    panic!(
                        "case {idx}: expected error for '{}', got Ok('{}')",
                        c.input, actual
                    );
                }
                (Err(err), Ok(exp)) => {
                    panic!(
                        "case {idx}: expected Ok('{}') for '{}', got Err('{}')",
                        exp, c.input, err
                    );
                }
            }
        }
    }

    #[test]
    fn hostname_length_limits() {
        // Label > 63 chars
        let long_label = format!("{}.example.com", "a".repeat(64));
        assert!(normalize_hostname(&long_label, false).is_err());

        // Label == 63 chars (valid)
        let exact_label = format!("{}.example.com", "a".repeat(63));
        assert!(normalize_hostname(&exact_label, false).is_ok());

        // Total > 253 chars
        let repeated = "a".repeat(50);
        let long_host = format!("{repeated}.{repeated}.{repeated}.{repeated}.{repeated}.com");
        assert!(long_host.len() > 253);
        assert!(normalize_hostname(&long_host, false).is_err());
    }
}
