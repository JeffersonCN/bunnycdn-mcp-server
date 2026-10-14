//! Redaction of sensitive API fields (C3). Applied to every raw/`verbose` output.

use serde_json::Value;

pub const REDACTED: &str = "[redacted]";

/// Key names that are always sensitive (compared case-insensitively).
const EXACT: &[&str] = &[
    "ZoneSecurityKey",
    "AWSSigningKey",
    "AWSSigningSecret",
    "LogForwardingToken",
    // Hostnames[].Certificate / CertificateKey: redacted wherever they appear, which is a
    // superset of the contract and costs nothing (no other field uses these names).
    "Certificate",
    "CertificateKey",
];

/// Case-insensitive substrings that mark a key as sensitive (`apikey|accesskey|securitykey`: CR-2
/// on T02 — `ApiKey`, `ApiAccessKey`, `ReadOnlyApiKey`, `SecurityKey` in the API spec).
const CONTAINS: &[&str] = &[
    "secret",
    "password",
    "token",
    "privatekey",
    "apikey",
    "accesskey",
    "securitykey",
];

/// Recursively replaces sensitive values with `"[redacted]"`.
///
/// Booleans, numbers and nulls under substring-matched keys are kept: they cannot carry a
/// secret, and keys like `EnableTokenAuthentication` are useful configuration flags (CR-1 on T02
/// proposes this wording for C3).
pub fn redact(v: Value) -> Value {
    match v {
        Value::Object(map) => Value::Object(
            map.into_iter()
                .map(|(k, v)| {
                    let v = if is_sensitive(&k, &v) {
                        Value::String(REDACTED.to_string())
                    } else {
                        redact(v)
                    };
                    (k, v)
                })
                .collect(),
        ),
        Value::Array(items) => Value::Array(items.into_iter().map(redact).collect()),
        other => other,
    }
}

fn is_sensitive(key: &str, value: &Value) -> bool {
    if value.is_null() {
        return false;
    }
    if EXACT.iter().any(|name| name.eq_ignore_ascii_case(key)) {
        return true;
    }
    if matches!(value, Value::Bool(_) | Value::Number(_)) {
        return false;
    }
    let lower = key.to_ascii_lowercase();
    CONTAINS.iter().any(|needle| lower.contains(needle))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn redacts_exact_and_substring_keys() {
        let v = redact(json!({
            "Id": 1,
            "ZoneSecurityKey": "zsk",
            "AWSSigningKey": "a",
            "AWSSigningSecret": "b",
            "LogForwardingToken": "c",
            "ReadOnlyPassword": "d",
            "SomePrivateKey": "e",
            "EnableTokenAuthentication": true,
            "MonthlyAiGatewayInputTokens": 42,
            "ApiSecret": null,
            "Name": "zone"
        }));
        assert_eq!(
            v,
            json!({
                "Id": 1,
                "ZoneSecurityKey": REDACTED,
                "AWSSigningKey": REDACTED,
                "AWSSigningSecret": REDACTED,
                "LogForwardingToken": REDACTED,
                "ReadOnlyPassword": REDACTED,
                "SomePrivateKey": REDACTED,
                "EnableTokenAuthentication": true,
                "MonthlyAiGatewayInputTokens": 42,
                "ApiSecret": null,
                "Name": "zone"
            })
        );
    }

    #[test]
    fn redacts_api_access_and_security_keys() {
        let v = redact(json!({
            "ApiKey": "a",
            "ApiAccessKey": "b",
            "ReadOnlyApiKey": "c",
            "SecurityKey": "d",
            "Nested": [{"accessKey": "e"}],
            "EnableApiKeyThing": false,
            "KeyName": "kept"
        }));
        assert_eq!(
            v,
            json!({
                "ApiKey": REDACTED,
                "ApiAccessKey": REDACTED,
                "ReadOnlyApiKey": REDACTED,
                "SecurityKey": REDACTED,
                "Nested": [{"accessKey": REDACTED}],
                "EnableApiKeyThing": false,
                "KeyName": "kept"
            })
        );
    }

    #[test]
    fn redacts_nested_objects_and_arrays() {
        let v = redact(json!({
            "Hostnames": [
                {"Value": "a.example", "Certificate": "PEM", "CertificateKey": "KEY", "HasCertificate": true},
                {"Value": "b.example"}
            ],
            "Nested": {"Deeper": [{"Token": "t", "Keep": "x"}]}
        }));
        assert_eq!(
            v,
            json!({
                "Hostnames": [
                    {"Value": "a.example", "Certificate": REDACTED, "CertificateKey": REDACTED, "HasCertificate": true},
                    {"Value": "b.example"}
                ],
                "Nested": {"Deeper": [{"Token": REDACTED, "Keep": "x"}]}
            })
        );
    }

    #[test]
    fn exact_keys_ignore_case() {
        let v = redact(json!({"zoneSecurityKey": "a", "CERTIFICATE": "b", "awssigningkey": "c"}));
        assert_eq!(
            v,
            json!({"zoneSecurityKey": REDACTED, "CERTIFICATE": REDACTED, "awssigningkey": REDACTED})
        );
    }

    #[test]
    fn sensitive_containers_are_redacted_whole() {
        let v = redact(json!({"Secrets": {"a": "b"}, "Tokens": ["x"]}));
        assert_eq!(v, json!({"Secrets": REDACTED, "Tokens": REDACTED}));
    }

    #[test]
    fn scalars_pass_through() {
        assert_eq!(redact(json!("Token")), json!("Token"));
        assert_eq!(redact(json!([1, "a"])), json!([1, "a"]));
    }
}
