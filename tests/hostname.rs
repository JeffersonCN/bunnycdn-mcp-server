//! Integration tests for Hostnames & SSL tools (specs/tools/hostname.md, REQ-HS-01..05).

mod common;

use base64::Engine;
use base64::prelude::BASE64_STANDARD;
use bunnycdn_mcp::tools::hostname::{
    CertificateAddArgs, CertificateIssueFreeArgs, CertificateRemoveArgs, HostnameAddArgs,
    HostnameForceSslArgs, HostnameRemoveArgs,
};
use rmcp::handler::server::wrapper::Parameters;
use serde_json::json;
use wiremock::matchers::{body_json, header, method, path, query_param};
use wiremock::{Mock, ResponseTemplate};

use common::{TEST_KEY, assert_err, assert_ok, json as parse_json, setup, text};

const SAMPLE_CERT_PEM: &str = "-----BEGIN CERTIFICATE-----\nMIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8AMIIBCgKCAQEA...\n-----END CERTIFICATE-----";
const SAMPLE_KEY_PEM: &str =
    "-----BEGIN RSA PRIVATE KEY-----\nMIIEowIBAAKCAQEA...\n-----END RSA PRIVATE KEY-----";

// covers: TOOL-HS-01, REQ-HS-01
#[tokio::test]
async fn hostname_add_success() {
    let (mock, srv) = setup().await;

    Mock::given(method("POST"))
        .and(path("/pullzone/123/addHostname"))
        .and(header("AccessKey", TEST_KEY))
        .and(body_json(json!({ "Hostname": "cdn.example.com" })))
        .respond_with(ResponseTemplate::new(204))
        .expect(1)
        .mount(&mock)
        .await;

    let res = srv
        .bunny_hostname_add(Parameters(HostnameAddArgs {
            id: 123,
            hostname: "cdn.example.com".to_string(),
        }))
        .await
        .unwrap();

    assert_ok(&res);
    let out = parse_json(&res);
    assert_eq!(out["ok"], true);
    assert_eq!(out["id"], 123);
    assert_eq!(out["hostname"], "cdn.example.com");
    assert_eq!(
        out["next"],
        "run bunny_certificate_issue_free to enable HTTPS"
    );
}

// covers: TOOL-HS-01, REQ-HS-01
#[tokio::test]
async fn hostname_add_normalizes_url_and_casing() {
    let (mock, srv) = setup().await;

    Mock::given(method("POST"))
        .and(path("/pullzone/456/addHostname"))
        .and(header("AccessKey", TEST_KEY))
        .and(body_json(json!({ "Hostname": "cdn.example.com" })))
        .respond_with(ResponseTemplate::new(204))
        .expect(1)
        .mount(&mock)
        .await;

    let res = srv
        .bunny_hostname_add(Parameters(HostnameAddArgs {
            id: 456,
            hostname: "  https://CDN.EXAMPLE.COM:443/some/path?foo=1  ".to_string(),
        }))
        .await
        .unwrap();

    assert_ok(&res);
    let out = parse_json(&res);
    assert_eq!(out["ok"], true);
    assert_eq!(out["id"], 456);
    assert_eq!(out["hostname"], "cdn.example.com");
}

// covers: TOOL-HS-01, REQ-HS-01
#[tokio::test]
async fn hostname_add_validation_failures() {
    let (_mock, srv) = setup().await;

    // id == 0
    let res = srv
        .bunny_hostname_add(Parameters(HostnameAddArgs {
            id: 0,
            hostname: "cdn.example.com".to_string(),
        }))
        .await
        .unwrap();
    assert_err(&res, "id must be >= 1");

    // invalid scheme (http://)
    let res = srv
        .bunny_hostname_add(Parameters(HostnameAddArgs {
            id: 123,
            hostname: "http://cdn.example.com".to_string(),
        }))
        .await
        .unwrap();
    assert_err(&res, "Invalid argument");

    // wildcard not permitted for addHostname
    let res = srv
        .bunny_hostname_add(Parameters(HostnameAddArgs {
            id: 123,
            hostname: "*.example.com".to_string(),
        }))
        .await
        .unwrap();
    assert_err(&res, "Invalid argument");
}

// covers: TOOL-HS-01, REQ-HS-01
#[tokio::test]
async fn hostname_add_api_error() {
    let (mock, srv) = setup().await;

    Mock::given(method("POST"))
        .and(path("/pullzone/123/addHostname"))
        .respond_with(ResponseTemplate::new(400).set_body_json(json!({
            "ErrorKey": "pullzone.hostname_already_exists",
            "Message": "The hostname is already linked to another Pull Zone"
        })))
        .expect(1)
        .mount(&mock)
        .await;

    let res = srv
        .bunny_hostname_add(Parameters(HostnameAddArgs {
            id: 123,
            hostname: "cdn.example.com".to_string(),
        }))
        .await
        .unwrap();

    assert_err(&res, "400");
    assert_err(&res, "pullzone.hostname_already_exists");
}

// covers: TOOL-HS-02, REQ-HS-01
#[tokio::test]
async fn hostname_remove_success() {
    let (mock, srv) = setup().await;

    Mock::given(method("DELETE"))
        .and(path("/pullzone/123/removeHostname"))
        .and(header("AccessKey", TEST_KEY))
        .and(body_json(json!({ "Hostname": "cdn.example.com" })))
        .respond_with(ResponseTemplate::new(204))
        .expect(1)
        .mount(&mock)
        .await;

    let res = srv
        .bunny_hostname_remove(Parameters(HostnameRemoveArgs {
            id: 123,
            hostname: "cdn.example.com".to_string(),
        }))
        .await
        .unwrap();

    assert_ok(&res);
    let out = parse_json(&res);
    assert_eq!(out["ok"], true);
    assert_eq!(out["id"], 123);
    assert_eq!(out["hostname"], "cdn.example.com");
}

// covers: TOOL-HS-02, REQ-HS-01
#[tokio::test]
async fn hostname_remove_validation_and_api_error() {
    let (mock, srv) = setup().await;

    // Local validation: id == 0
    let res = srv
        .bunny_hostname_remove(Parameters(HostnameRemoveArgs {
            id: 0,
            hostname: "cdn.example.com".to_string(),
        }))
        .await
        .unwrap();
    assert_err(&res, "id must be >= 1");

    // API error 404
    Mock::given(method("DELETE"))
        .and(path("/pullzone/999/removeHostname"))
        .respond_with(ResponseTemplate::new(404))
        .expect(1)
        .mount(&mock)
        .await;

    let res = srv
        .bunny_hostname_remove(Parameters(HostnameRemoveArgs {
            id: 999,
            hostname: "cdn.example.com".to_string(),
        }))
        .await
        .unwrap();

    assert_err(&res, "404");
}

// covers: TOOL-HS-03, REQ-HS-02
#[tokio::test]
async fn hostname_force_ssl_success_and_api_error() {
    let (mock, srv) = setup().await;

    Mock::given(method("POST"))
        .and(path("/pullzone/123/setForceSSL"))
        .and(header("AccessKey", TEST_KEY))
        .and(body_json(
            json!({ "Hostname": "cdn.example.com", "ForceSSL": true }),
        ))
        .respond_with(ResponseTemplate::new(204))
        .expect(1)
        .mount(&mock)
        .await;

    let res = srv
        .bunny_hostname_force_ssl(Parameters(HostnameForceSslArgs {
            id: 123,
            hostname: "cdn.example.com".to_string(),
            enabled: true,
        }))
        .await
        .unwrap();

    assert_ok(&res);
    let out = parse_json(&res);
    assert_eq!(out["ok"], true);
    assert_eq!(out["id"], 123);
    assert_eq!(out["hostname"], "cdn.example.com");
    assert_eq!(out["force_ssl"], true);

    // Disable force SSL
    Mock::given(method("POST"))
        .and(path("/pullzone/123/setForceSSL"))
        .and(header("AccessKey", TEST_KEY))
        .and(body_json(
            json!({ "Hostname": "cdn.example.com", "ForceSSL": false }),
        ))
        .respond_with(ResponseTemplate::new(204))
        .expect(1)
        .mount(&mock)
        .await;

    let res = srv
        .bunny_hostname_force_ssl(Parameters(HostnameForceSslArgs {
            id: 123,
            hostname: "cdn.example.com".to_string(),
            enabled: false,
        }))
        .await
        .unwrap();

    assert_ok(&res);
    let out = parse_json(&res);
    assert_eq!(out["force_ssl"], false);

    // Validation failure (id == 0)
    let res = srv
        .bunny_hostname_force_ssl(Parameters(HostnameForceSslArgs {
            id: 0,
            hostname: "cdn.example.com".to_string(),
            enabled: true,
        }))
        .await
        .unwrap();
    assert_err(&res, "id must be >= 1");
}

// covers: TOOL-HS-04, REQ-HS-03
#[tokio::test]
async fn certificate_issue_free_success() {
    let (mock, srv) = setup().await;

    Mock::given(method("GET"))
        .and(path("/pullzone/loadFreeCertificate"))
        .and(query_param("hostname", "cdn.example.com"))
        .and(header("AccessKey", TEST_KEY))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "Id": 123 })))
        .expect(1)
        .mount(&mock)
        .await;

    let res = srv
        .bunny_certificate_issue_free(Parameters(CertificateIssueFreeArgs {
            hostname: "cdn.example.com".to_string(),
        }))
        .await
        .unwrap();

    assert_ok(&res);
    let out = parse_json(&res);
    assert_eq!(out["ok"], true);
    assert_eq!(out["hostname"], "cdn.example.com");
}

// covers: TOOL-HS-04, REQ-HS-03
#[tokio::test]
async fn certificate_issue_free_dns_propagating_400_appends_cname_hint() {
    let (mock, srv) = setup().await;

    Mock::given(method("GET"))
        .and(path("/pullzone/loadFreeCertificate"))
        .and(query_param("hostname", "cdn.example.com"))
        .respond_with(ResponseTemplate::new(400).set_body_json(json!({
            "ErrorKey": "pullzone.load_free_certificate_failed",
            "Message": "Let's Encrypt validation failed"
        })))
        .expect(1)
        .mount(&mock)
        .await;

    let res = srv
        .bunny_certificate_issue_free(Parameters(CertificateIssueFreeArgs {
            hostname: "cdn.example.com".to_string(),
        }))
        .await
        .unwrap();

    assert_err(&res, "400");
    assert_err(
        &res,
        "Check that cdn.example.com has a CNAME to the zone's b-cdn.net hostname and retry in a few minutes.",
    );
}

// covers: TOOL-HS-04, REQ-HS-03
#[tokio::test]
async fn certificate_issue_free_other_api_error() {
    let (mock, srv) = setup().await;

    Mock::given(method("GET"))
        .and(path("/pullzone/loadFreeCertificate"))
        .respond_with(ResponseTemplate::new(401))
        .expect(1)
        .mount(&mock)
        .await;

    let res = srv
        .bunny_certificate_issue_free(Parameters(CertificateIssueFreeArgs {
            hostname: "cdn.example.com".to_string(),
        }))
        .await
        .unwrap();

    assert_err(&res, "401");
    // Should NOT contain the CNAME hint on non-400 errors
    assert!(!text(&res).contains("CNAME"));
}

// covers: TOOL-HS-05, REQ-HS-04
#[tokio::test]
async fn certificate_add_success_with_base64_and_wildcard() {
    let (mock, srv) = setup().await;

    let expected_cert_b64 = BASE64_STANDARD.encode(SAMPLE_CERT_PEM.as_bytes());
    let expected_key_b64 = BASE64_STANDARD.encode(SAMPLE_KEY_PEM.as_bytes());

    Mock::given(method("POST"))
        .and(path("/pullzone/123/addCertificate"))
        .and(header("AccessKey", TEST_KEY))
        .and(body_json(json!({
            "Hostname": "*.example.com",
            "Certificate": expected_cert_b64,
            "CertificateKey": expected_key_b64,
        })))
        .respond_with(ResponseTemplate::new(204))
        .expect(1)
        .mount(&mock)
        .await;

    let res = srv
        .bunny_certificate_add(Parameters(CertificateAddArgs {
            id: 123,
            hostname: "*.example.com".to_string(),
            certificate_pem: SAMPLE_CERT_PEM.to_string(),
            private_key_pem: SAMPLE_KEY_PEM.to_string(),
        }))
        .await
        .unwrap();

    assert_ok(&res);
    let out = parse_json(&res);
    assert_eq!(out["ok"], true);
    assert_eq!(out["id"], 123);
    assert_eq!(out["hostname"], "*.example.com");
}

// covers: TOOL-HS-05, REQ-HS-04
#[tokio::test]
async fn certificate_add_invalid_pem_does_not_echo_secret() {
    let (_mock, srv) = setup().await;

    let sensitive_key = "MY_SUPER_SECRET_RAW_KEY_DATA";

    // Invalid certificate PEM
    let res = srv
        .bunny_certificate_add(Parameters(CertificateAddArgs {
            id: 123,
            hostname: "cdn.example.com".to_string(),
            certificate_pem: "invalid-cert".to_string(),
            private_key_pem: format!(
                "-----BEGIN PRIVATE KEY-----\n{sensitive_key}\n-----END PRIVATE KEY-----"
            ),
        }))
        .await
        .unwrap();

    assert_err(&res, "certificate_pem must be PEM-encoded");
    let err_text = text(&res);
    assert!(
        !err_text.contains(sensitive_key),
        "Error message must not echo sensitive key data"
    );

    // Invalid private key PEM
    let sensitive_cert = "MY_CERT_DATA";
    let res = srv
        .bunny_certificate_add(Parameters(CertificateAddArgs {
            id: 123,
            hostname: "cdn.example.com".to_string(),
            certificate_pem: format!(
                "-----BEGIN CERTIFICATE-----\n{sensitive_cert}\n-----END CERTIFICATE-----"
            ),
            private_key_pem: "invalid-key".to_string(),
        }))
        .await
        .unwrap();

    assert_err(&res, "private_key_pem must be PEM-encoded");
    let err_text = text(&res);
    assert!(
        !err_text.contains(sensitive_cert),
        "Error message must not echo cert data"
    );
}

// covers: TOOL-HS-05, REQ-HS-04
#[tokio::test]
async fn certificate_add_api_error() {
    let (mock, srv) = setup().await;

    Mock::given(method("POST"))
        .and(path("/pullzone/123/addCertificate"))
        .respond_with(ResponseTemplate::new(400).set_body_json(json!({
            "ErrorKey": "pullzone.certificate_invalid",
            "Message": "The uploaded certificate is invalid or does not match the key"
        })))
        .expect(1)
        .mount(&mock)
        .await;

    let res = srv
        .bunny_certificate_add(Parameters(CertificateAddArgs {
            id: 123,
            hostname: "cdn.example.com".to_string(),
            certificate_pem: SAMPLE_CERT_PEM.to_string(),
            private_key_pem: SAMPLE_KEY_PEM.to_string(),
        }))
        .await
        .unwrap();

    assert_err(&res, "400");
    assert_err(&res, "pullzone.certificate_invalid");
}

// covers: TOOL-HS-06, REQ-HS-05
#[tokio::test]
async fn certificate_remove_success() {
    let (mock, srv) = setup().await;

    Mock::given(method("DELETE"))
        .and(path("/pullzone/123/removeCertificate"))
        .and(header("AccessKey", TEST_KEY))
        .and(body_json(json!({ "Hostname": "cdn.example.com" })))
        .respond_with(ResponseTemplate::new(204))
        .expect(1)
        .mount(&mock)
        .await;

    let res = srv
        .bunny_certificate_remove(Parameters(CertificateRemoveArgs {
            id: 123,
            hostname: "cdn.example.com".to_string(),
        }))
        .await
        .unwrap();

    assert_ok(&res);
    let out = parse_json(&res);
    assert_eq!(out["ok"], true);
    assert_eq!(out["id"], 123);
    assert_eq!(out["hostname"], "cdn.example.com");
}

// covers: TOOL-HS-06, REQ-HS-05
#[tokio::test]
async fn certificate_remove_wildcard_not_allowed_and_api_error() {
    let (mock, srv) = setup().await;

    // Wildcard is disallowed in certificate_remove (only allowed in certificate_add per spec)
    let res = srv
        .bunny_certificate_remove(Parameters(CertificateRemoveArgs {
            id: 123,
            hostname: "*.example.com".to_string(),
        }))
        .await
        .unwrap();
    assert_err(&res, "Invalid argument");

    // id == 0
    let res = srv
        .bunny_certificate_remove(Parameters(CertificateRemoveArgs {
            id: 0,
            hostname: "cdn.example.com".to_string(),
        }))
        .await
        .unwrap();
    assert_err(&res, "id must be >= 1");

    // 404 API error
    Mock::given(method("DELETE"))
        .and(path("/pullzone/123/removeCertificate"))
        .respond_with(ResponseTemplate::new(404))
        .expect(1)
        .mount(&mock)
        .await;

    let res = srv
        .bunny_certificate_remove(Parameters(CertificateRemoveArgs {
            id: 123,
            hostname: "cdn.example.com".to_string(),
        }))
        .await
        .unwrap();
    assert_err(&res, "404");
}
