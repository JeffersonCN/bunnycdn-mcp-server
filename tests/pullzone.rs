//! Tests for Pull Zone read tools (TOOL-PZ-01, TOOL-PZ-02, TOOL-PZ-06).

mod common;

use bunnycdn_mcp::tools::pullzone::{
    PullZoneCheckAvailabilityArgs, PullZoneGetArgs, PullZoneListArgs,
};
use rmcp::handler::server::wrapper::Parameters;
use serde_json::json;
use wiremock::matchers::{body_json, header, method, path, query_param};
use wiremock::{Mock, ResponseTemplate};

use common::{TEST_KEY, assert_err, assert_ok, fixture, json, setup};

// covers: TOOL-PZ-01
#[tokio::test]
async fn pullzone_list_array_response() {
    let (mock, srv) = setup().await;
    let single = fixture("pullzone_get");

    Mock::given(method("GET"))
        .and(path("/pullzone"))
        .and(query_param("page", "1"))
        .and(query_param("perPage", "100"))
        .and(query_param("includeCertificate", "false"))
        .and(header("AccessKey", TEST_KEY))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([single])))
        .expect(1)
        .mount(&mock)
        .await;

    let args = PullZoneListArgs {
        search: None,
        page: 1,
        per_page: 100,
    };
    let r = srv.bunny_pullzone_list(Parameters(args)).await.unwrap();
    assert_ok(&r);
    let val = json(&r);

    assert_eq!(val["page"], 1);
    assert_eq!(val["per_page"], 100);
    assert_eq!(val["total"], 1);
    assert_eq!(val["has_more"], false);
    assert_eq!(val["items"].as_array().unwrap().len(), 1);
    assert_eq!(val["items"][0]["id"], 123);
    assert_eq!(val["items"][0]["name"], "assets");
    assert_eq!(val["items"][0]["tier"], "standard");
    assert_eq!(val["items"][0]["origin"], "https://origin.example.com");
    assert_eq!(
        val["items"][0]["hostnames"],
        json!(["cdn.example.com", "assets.b-cdn.net"])
    );
    assert_eq!(val["items"][0]["monthly_bandwidth_used"], 123456789);
}

// covers: TOOL-PZ-01
#[tokio::test]
async fn pullzone_list_paginated_response() {
    let (mock, srv) = setup().await;
    let single = fixture("pullzone_get");

    Mock::given(method("GET"))
        .and(path("/pullzone"))
        .and(query_param("page", "2"))
        .and(query_param("perPage", "50"))
        .and(query_param("includeCertificate", "false"))
        .and(header("AccessKey", TEST_KEY))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "Items": [single],
            "CurrentPage": 2,
            "TotalItems": 7,
            "HasMoreItems": false
        })))
        .expect(1)
        .mount(&mock)
        .await;

    let args = PullZoneListArgs {
        search: None,
        page: 2,
        per_page: 50,
    };
    let r = srv.bunny_pullzone_list(Parameters(args)).await.unwrap();
    assert_ok(&r);
    let val = json(&r);

    assert_eq!(val["page"], 2);
    assert_eq!(val["per_page"], 50);
    assert_eq!(val["total"], 7);
    assert_eq!(val["has_more"], false);
    assert_eq!(val["items"].as_array().unwrap().len(), 1);
}

// covers: TOOL-PZ-01
#[tokio::test]
async fn pullzone_list_search_query() {
    let (mock, srv) = setup().await;

    Mock::given(method("GET"))
        .and(path("/pullzone"))
        .and(query_param("page", "1"))
        .and(query_param("perPage", "100"))
        .and(query_param("search", "my-zone"))
        .and(query_param("includeCertificate", "false"))
        .and(header("AccessKey", TEST_KEY))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([])))
        .expect(1)
        .mount(&mock)
        .await;

    let args = PullZoneListArgs {
        search: Some("my-zone".to_string()),
        page: 1,
        per_page: 100,
    };
    let r = srv.bunny_pullzone_list(Parameters(args)).await.unwrap();
    assert_ok(&r);
    let val = json(&r);
    assert_eq!(val["items"], json!([]));
    assert_eq!(val["total"], 0);
}

// covers: TOOL-PZ-01
#[tokio::test]
async fn pullzone_list_unauthorized() {
    let (mock, srv) = setup().await;

    Mock::given(method("GET"))
        .and(path("/pullzone"))
        .respond_with(ResponseTemplate::new(401))
        .mount(&mock)
        .await;

    let args = PullZoneListArgs {
        search: None,
        page: 1,
        per_page: 100,
    };
    let r = srv.bunny_pullzone_list(Parameters(args)).await.unwrap();
    assert_err(&r, "Check BUNNY_API_KEY");
}

// covers: TOOL-PZ-01
#[tokio::test]
async fn pullzone_list_invalid_args() {
    let (_, srv) = setup().await;

    let args = PullZoneListArgs {
        search: None,
        page: 0,
        per_page: 100,
    };
    let r = srv.bunny_pullzone_list(Parameters(args)).await.unwrap();
    assert_err(&r, "page must be >= 1");

    let args = PullZoneListArgs {
        search: None,
        page: 1,
        per_page: 4,
    };
    let r = srv.bunny_pullzone_list(Parameters(args)).await.unwrap();
    assert_err(&r, "per_page must be between 5 and 1000");

    let args = PullZoneListArgs {
        search: None,
        page: 1,
        per_page: 1001,
    };
    let r = srv.bunny_pullzone_list(Parameters(args)).await.unwrap();
    assert_err(&r, "per_page must be between 5 and 1000");
}

// covers: TOOL-PZ-02
#[tokio::test]
async fn pullzone_get_summary_matches_snapshot() {
    let (mock, srv) = setup().await;
    let fixture_val = fixture("pullzone_get");

    Mock::given(method("GET"))
        .and(path("/pullzone/123"))
        .and(header("AccessKey", TEST_KEY))
        .respond_with(ResponseTemplate::new(200).set_body_json(fixture_val))
        .expect(1)
        .mount(&mock)
        .await;

    let args = PullZoneGetArgs {
        id: 123,
        verbose: false,
    };
    let r = srv.bunny_pullzone_get(Parameters(args)).await.unwrap();
    assert_ok(&r);
    let val = json(&r);

    insta::assert_json_snapshot!(val);
}

// covers: TOOL-PZ-02
#[tokio::test]
async fn pullzone_get_verbose_redacts_sensitive_keys() {
    let (mock, srv) = setup().await;
    let fixture_val = fixture("pullzone_get");

    Mock::given(method("GET"))
        .and(path("/pullzone/123"))
        .and(header("AccessKey", TEST_KEY))
        .respond_with(ResponseTemplate::new(200).set_body_json(fixture_val))
        .expect(1)
        .mount(&mock)
        .await;

    let args = PullZoneGetArgs {
        id: 123,
        verbose: true,
    };
    let r = srv.bunny_pullzone_get(Parameters(args)).await.unwrap();
    assert_ok(&r);
    let val = json(&r);

    // Verify sensitive keys are redacted
    assert_eq!(val["ZoneSecurityKey"], "[redacted]");
    assert_eq!(val["AWSSigningKey"], "[redacted]");
    assert_eq!(val["AWSSigningSecret"], "[redacted]");
    assert_eq!(val["LogForwardingToken"], "[redacted]");
    assert_eq!(val["Hostnames"][0]["Certificate"], "[redacted]");
    assert_eq!(val["Hostnames"][0]["CertificateKey"], "[redacted]");

    // Verify non-sensitive fields are untouched
    assert_eq!(val["Id"], 123);
    assert_eq!(val["Name"], "assets");
    assert_eq!(val["OriginUrl"], "https://origin.example.com");
}

// covers: TOOL-PZ-02
#[tokio::test]
async fn pullzone_get_not_found() {
    let (mock, srv) = setup().await;

    Mock::given(method("GET"))
        .and(path("/pullzone/999"))
        .respond_with(ResponseTemplate::new(404))
        .expect(1)
        .mount(&mock)
        .await;

    let args = PullZoneGetArgs {
        id: 999,
        verbose: false,
    };
    let r = srv.bunny_pullzone_get(Parameters(args)).await.unwrap();
    assert_err(&r, "bunny_pullzone_list");
}

// covers: TOOL-PZ-02
#[tokio::test]
async fn pullzone_get_invalid_id() {
    let (_, srv) = setup().await;

    let args = PullZoneGetArgs {
        id: 0,
        verbose: false,
    };
    let r = srv.bunny_pullzone_get(Parameters(args)).await.unwrap();
    assert_err(&r, "id must be >= 1");
}

// covers: TOOL-PZ-06
#[tokio::test]
async fn pullzone_check_availability_available() {
    let (mock, srv) = setup().await;

    Mock::given(method("POST"))
        .and(path("/pullzone/checkavailability"))
        .and(header("AccessKey", TEST_KEY))
        .and(body_json(json!({ "Name": "my-new-zone" })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "Available": true
        })))
        .expect(1)
        .mount(&mock)
        .await;

    let args = PullZoneCheckAvailabilityArgs {
        name: "my-new-zone".to_string(),
    };
    let r = srv
        .bunny_pullzone_check_availability(Parameters(args))
        .await
        .unwrap();
    assert_ok(&r);
    let val = json(&r);
    assert_eq!(
        val,
        json!({
            "name": "my-new-zone",
            "available": true
        })
    );
}

// covers: TOOL-PZ-06
#[tokio::test]
async fn pullzone_check_availability_unavailable() {
    let (mock, srv) = setup().await;

    Mock::given(method("POST"))
        .and(path("/pullzone/checkavailability"))
        .and(header("AccessKey", TEST_KEY))
        .and(body_json(json!({ "Name": "existing-zone" })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "Available": false
        })))
        .expect(1)
        .mount(&mock)
        .await;

    let args = PullZoneCheckAvailabilityArgs {
        name: "existing-zone".to_string(),
    };
    let r = srv
        .bunny_pullzone_check_availability(Parameters(args))
        .await
        .unwrap();
    assert_ok(&r);
    let val = json(&r);
    assert_eq!(
        val,
        json!({
            "name": "existing-zone",
            "available": false
        })
    );
}

// covers: TOOL-PZ-06
#[tokio::test]
async fn pullzone_check_availability_missing_available_is_decode_error() {
    let (mock, srv) = setup().await;

    Mock::given(method("POST"))
        .and(path("/pullzone/checkavailability"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({})))
        .expect(1)
        .mount(&mock)
        .await;

    let args = PullZoneCheckAvailabilityArgs {
        name: "zone".to_string(),
    };
    let r = srv
        .bunny_pullzone_check_availability(Parameters(args))
        .await
        .unwrap();
    assert_err(&r, "response missing Available field");
}

// covers: TOOL-PZ-06
#[tokio::test]
async fn pullzone_check_availability_api_error() {
    let (mock, srv) = setup().await;

    Mock::given(method("POST"))
        .and(path("/pullzone/checkavailability"))
        .respond_with(ResponseTemplate::new(400))
        .expect(1)
        .mount(&mock)
        .await;

    let args = PullZoneCheckAvailabilityArgs {
        name: "zone".to_string(),
    };
    let r = srv
        .bunny_pullzone_check_availability(Parameters(args))
        .await
        .unwrap();
    assert_err(&r, "400");
}

// covers: TOOL-PZ-06
#[tokio::test]
async fn pullzone_check_availability_invalid_name() {
    let (_, srv) = setup().await;

    let args = PullZoneCheckAvailabilityArgs {
        name: "   ".to_string(),
    };
    let r = srv
        .bunny_pullzone_check_availability(Parameters(args))
        .await
        .unwrap();
    assert_err(&r, "name cannot be empty");
}
