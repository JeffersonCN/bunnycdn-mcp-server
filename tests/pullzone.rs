//! Tests for Pull Zone read and write tools (TOOL-PZ-01..07).

mod common;

use bunnycdn_mcp::tools::pullzone::{
    PullZoneCheckAvailabilityArgs, PullZoneCreateArgs, PullZoneDeleteArgs, PullZoneGetArgs,
    PullZoneListArgs, PullZoneResetSecurityKeyArgs, PullZoneUpdateArgs,
};
use rmcp::handler::server::wrapper::Parameters;
use serde_json::json;
use wiremock::matchers::{body_json, header, method, path, query_param};
use wiremock::{Mock, ResponseTemplate};

use common::{TEST_KEY, assert_err, assert_ok, fixture, json, setup, text};

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

// covers: TOOL-PZ-03
#[tokio::test]
async fn pullzone_create_origin_url_success() {
    let (mock, srv) = setup().await;
    let single = fixture("pullzone_get");

    Mock::given(method("POST"))
        .and(path("/pullzone"))
        .and(header("AccessKey", TEST_KEY))
        .and(body_json(json!({
            "Name": "my-assets",
            "OriginUrl": "https://origin.example.com",
            "Type": 0
        })))
        .respond_with(ResponseTemplate::new(201).set_body_json(single))
        .expect(1)
        .mount(&mock)
        .await;

    let args = PullZoneCreateArgs {
        name: "my-assets".to_string(),
        origin_url: Some("https://origin.example.com".to_string()),
        storage_zone_id: None,
        tier: None,
        settings: None,
    };
    let r = srv.bunny_pullzone_create(Parameters(args)).await.unwrap();
    assert_ok(&r);
    let val = json(&r);
    assert_eq!(val["id"], 123);
    assert_eq!(val["name"], "assets");
    assert_eq!(val["origin"], "https://origin.example.com");
}

// covers: TOOL-PZ-03
#[tokio::test]
async fn pullzone_create_storage_origin_success_with_empty_response() {
    let (mock, srv) = setup().await;

    Mock::given(method("POST"))
        .and(path("/pullzone"))
        .and(header("AccessKey", TEST_KEY))
        .and(body_json(json!({
            "Name": "my-storage-zone",
            "StorageZoneId": 789,
            "OriginType": 2,
            "Type": 1,
            "EnableSmartCache": true
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({})))
        .expect(1)
        .mount(&mock)
        .await;

    let mut settings = serde_json::Map::new();
    settings.insert("EnableSmartCache".to_string(), json!(true));

    let args = PullZoneCreateArgs {
        name: "my-storage-zone".to_string(),
        origin_url: None,
        storage_zone_id: Some(789),
        tier: Some("volume".to_string()),
        settings: Some(settings),
    };
    let r = srv.bunny_pullzone_create(Parameters(args)).await.unwrap();
    assert_ok(&r);
    let val = json(&r);
    assert_eq!(val["ok"], true);
    assert_eq!(val["name"], "my-storage-zone");
    assert_eq!(
        val["hint"],
        "use bunny_pullzone_list search=my-storage-zone to get the id"
    );
}

// covers: TOOL-PZ-03
#[tokio::test]
async fn pullzone_create_both_or_neither_origin_invalid() {
    let (_, srv) = setup().await;

    // Both provided
    let args_both = PullZoneCreateArgs {
        name: "my-zone".to_string(),
        origin_url: Some("https://origin.example.com".to_string()),
        storage_zone_id: Some(789),
        tier: None,
        settings: None,
    };
    let r = srv
        .bunny_pullzone_create(Parameters(args_both))
        .await
        .unwrap();
    assert_err(&r, "exactly one of origin_url or storage_zone_id");

    // Neither provided
    let args_neither = PullZoneCreateArgs {
        name: "my-zone".to_string(),
        origin_url: None,
        storage_zone_id: None,
        tier: None,
        settings: None,
    };
    let r = srv
        .bunny_pullzone_create(Parameters(args_neither))
        .await
        .unwrap();
    assert_err(&r, "exactly one of origin_url or storage_zone_id");
}

// covers: TOOL-PZ-03
#[tokio::test]
async fn pullzone_create_invalid_name_and_origin_url() {
    let (_, srv) = setup().await;

    let args_invalid_name = PullZoneCreateArgs {
        name: "my_invalid_name!".to_string(),
        origin_url: Some("https://origin.example.com".to_string()),
        storage_zone_id: None,
        tier: None,
        settings: None,
    };
    let r = srv
        .bunny_pullzone_create(Parameters(args_invalid_name))
        .await
        .unwrap();
    assert_err(&r, "name must match ^[a-z0-9-]{3,}$");

    let args_short_name = PullZoneCreateArgs {
        name: "ab".to_string(),
        origin_url: Some("https://origin.example.com".to_string()),
        storage_zone_id: None,
        tier: None,
        settings: None,
    };
    let r = srv
        .bunny_pullzone_create(Parameters(args_short_name))
        .await
        .unwrap();
    assert_err(&r, "name must match ^[a-z0-9-]{3,}$");

    let args_invalid_url = PullZoneCreateArgs {
        name: "valid-name".to_string(),
        origin_url: Some("ftp://invalid.scheme".to_string()),
        storage_zone_id: None,
        tier: None,
        settings: None,
    };
    let r = srv
        .bunny_pullzone_create(Parameters(args_invalid_url))
        .await
        .unwrap();
    assert_err(&r, "origin_url must have http or https scheme");
}

// covers: TOOL-PZ-03
#[tokio::test]
async fn pullzone_create_forbidden_settings() {
    let (_, srv) = setup().await;

    let mut settings = serde_json::Map::new();
    settings.insert("OriginUrl".to_string(), json!("https://evil.com"));

    let args = PullZoneCreateArgs {
        name: "valid-name".to_string(),
        origin_url: Some("https://origin.example.com".to_string()),
        storage_zone_id: None,
        tier: None,
        settings: Some(settings),
    };
    let r = srv.bunny_pullzone_create(Parameters(args)).await.unwrap();
    assert_err(&r, "settings cannot contain \"OriginUrl\"");
}

// covers: TOOL-PZ-03
#[tokio::test]
async fn pullzone_create_api_error_400_with_error_key() {
    let (mock, srv) = setup().await;

    Mock::given(method("POST"))
        .and(path("/pullzone"))
        .and(header("AccessKey", TEST_KEY))
        .respond_with(ResponseTemplate::new(400).set_body_json(json!({
            "ErrorKey": "pullzone.name_already_taken",
            "Field": "Name",
            "Message": "The pull zone name is already taken"
        })))
        .expect(1)
        .mount(&mock)
        .await;

    let args = PullZoneCreateArgs {
        name: "taken-zone".to_string(),
        origin_url: Some("https://origin.example.com".to_string()),
        storage_zone_id: None,
        tier: None,
        settings: None,
    };
    let r = srv.bunny_pullzone_create(Parameters(args)).await.unwrap();
    assert_err(&r, "pullzone.name_already_taken");
    assert_err(&r, "The pull zone name is already taken");
}

// covers: TOOL-PZ-04
#[tokio::test]
async fn pullzone_update_unknown_setting_with_suggestion() {
    let (_, srv) = setup().await;

    let mut settings = serde_json::Map::new();
    settings.insert("EnableSmartCach".to_string(), json!(true));

    let args = PullZoneUpdateArgs { id: 123, settings };
    let r = srv.bunny_pullzone_update(Parameters(args)).await.unwrap();
    assert_err(
        &r,
        "unknown setting \"EnableSmartCach\" (did you mean \"EnableSmartCache\"?)",
    );
}

// covers: TOOL-PZ-04
#[tokio::test]
async fn pullzone_update_case_normalization() {
    let (mock, srv) = setup().await;
    let single = fixture("pullzone_get");

    Mock::given(method("POST"))
        .and(path("/pullzone/123"))
        .and(header("AccessKey", TEST_KEY))
        .and(body_json(json!({
            "EnableSmartCache": true
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(single))
        .expect(1)
        .mount(&mock)
        .await;

    let mut settings = serde_json::Map::new();
    settings.insert("enablesmartcache".to_string(), json!(true));

    let args = PullZoneUpdateArgs { id: 123, settings };
    let r = srv.bunny_pullzone_update(Parameters(args)).await.unwrap();
    assert_ok(&r);
    let val = json(&r);
    assert_eq!(val["id"], 123);
}

// covers: TOOL-PZ-04
#[tokio::test]
async fn pullzone_update_wrong_type() {
    let (_, srv) = setup().await;

    let mut settings = serde_json::Map::new();
    settings.insert("EnableSmartCache".to_string(), json!("not_a_bool"));

    let args = PullZoneUpdateArgs { id: 123, settings };
    let r = srv.bunny_pullzone_update(Parameters(args)).await.unwrap();
    assert_err(&r, "setting \"EnableSmartCache\" expects bool, got string");
}

// covers: TOOL-PZ-04
#[tokio::test]
async fn pullzone_update_enum_by_name() {
    let (mock, srv) = setup().await;
    let single = fixture("pullzone_get");

    Mock::given(method("POST"))
        .and(path("/pullzone/123"))
        .and(header("AccessKey", TEST_KEY))
        .and(body_json(json!({
            "TlsSecurityLevel": 1
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(single))
        .expect(1)
        .mount(&mock)
        .await;

    let mut settings = serde_json::Map::new();
    settings.insert("TlsSecurityLevel".to_string(), json!("Compatible"));

    let args = PullZoneUpdateArgs { id: 123, settings };
    let r = srv.bunny_pullzone_update(Parameters(args)).await.unwrap();
    assert_ok(&r);
    let val = json(&r);
    assert_eq!(val["id"], 123);
}

// covers: TOOL-PZ-04
#[tokio::test]
async fn pullzone_update_empty_response_triggers_get() {
    let (mock, srv) = setup().await;
    let single = fixture("pullzone_get");

    Mock::given(method("POST"))
        .and(path("/pullzone/123"))
        .and(header("AccessKey", TEST_KEY))
        .and(body_json(json!({
            "EnableSmartCache": true,
            "CacheControlMaxAgeOverride": 3600
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({})))
        .expect(1)
        .mount(&mock)
        .await;

    Mock::given(method("GET"))
        .and(path("/pullzone/123"))
        .and(header("AccessKey", TEST_KEY))
        .respond_with(ResponseTemplate::new(200).set_body_json(single))
        .expect(1)
        .mount(&mock)
        .await;

    let mut settings = serde_json::Map::new();
    settings.insert("EnableSmartCache".to_string(), json!(true));
    settings.insert("CacheControlMaxAgeOverride".to_string(), json!(3600));

    let args = PullZoneUpdateArgs { id: 123, settings };
    let r = srv.bunny_pullzone_update(Parameters(args)).await.unwrap();
    assert_ok(&r);
    let val = json(&r);
    assert_eq!(val["id"], 123);
    assert_eq!(
        val["updated"],
        json!(["CacheControlMaxAgeOverride", "EnableSmartCache"])
    );
}

// covers: TOOL-PZ-04
#[tokio::test]
async fn pullzone_update_invalid_id_or_empty_settings() {
    let (_, srv) = setup().await;

    let mut settings = serde_json::Map::new();
    settings.insert("EnableSmartCache".to_string(), json!(true));

    let args_zero_id = PullZoneUpdateArgs {
        id: 0,
        settings: settings.clone(),
    };
    let r = srv
        .bunny_pullzone_update(Parameters(args_zero_id))
        .await
        .unwrap();
    assert_err(&r, "id must be >= 1");

    let args_empty_settings = PullZoneUpdateArgs {
        id: 123,
        settings: serde_json::Map::new(),
    };
    let r = srv
        .bunny_pullzone_update(Parameters(args_empty_settings))
        .await
        .unwrap();
    assert_err(&r, "settings must contain at least 1 key");
}

// covers: TOOL-PZ-05
#[tokio::test]
async fn pullzone_delete_success_204() {
    let (mock, srv) = setup().await;

    Mock::given(method("DELETE"))
        .and(path("/pullzone/123"))
        .and(header("AccessKey", TEST_KEY))
        .respond_with(ResponseTemplate::new(204))
        .expect(1)
        .mount(&mock)
        .await;

    let args = PullZoneDeleteArgs { id: 123 };
    let r = srv.bunny_pullzone_delete(Parameters(args)).await.unwrap();
    assert_ok(&r);
    let val = json(&r);
    assert_eq!(val, json!({ "ok": true, "deleted_id": 123 }));
}

// covers: TOOL-PZ-05
#[tokio::test]
async fn pullzone_delete_not_found_404() {
    let (mock, srv) = setup().await;

    Mock::given(method("DELETE"))
        .and(path("/pullzone/999"))
        .and(header("AccessKey", TEST_KEY))
        .respond_with(ResponseTemplate::new(404))
        .expect(1)
        .mount(&mock)
        .await;

    let args = PullZoneDeleteArgs { id: 999 };
    let r = srv.bunny_pullzone_delete(Parameters(args)).await.unwrap();
    assert_err(&r, "Not found (404)");
}

// covers: TOOL-PZ-05
#[tokio::test]
async fn pullzone_delete_invalid_id() {
    let (_, srv) = setup().await;

    let args = PullZoneDeleteArgs { id: 0 };
    let r = srv.bunny_pullzone_delete(Parameters(args)).await.unwrap();
    assert_err(&r, "id must be >= 1");
}

// covers: TOOL-PZ-07
#[tokio::test]
async fn pullzone_reset_security_key_with_custom_key() {
    let (mock, srv) = setup().await;

    Mock::given(method("POST"))
        .and(path("/pullzone/123/resetSecurityKey"))
        .and(header("AccessKey", TEST_KEY))
        .and(body_json(
            json!({ "SecurityKey": "super-secret-custom-key" }),
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({})))
        .expect(1)
        .mount(&mock)
        .await;

    let args = PullZoneResetSecurityKeyArgs {
        id: 123,
        security_key: Some("super-secret-custom-key".to_string()),
    };
    let r = srv
        .bunny_pullzone_reset_security_key(Parameters(args))
        .await
        .unwrap();
    assert_ok(&r);
    let val = json(&r);
    assert_eq!(
        val,
        json!({
            "ok": true,
            "id": 123,
            "note": "new key is visible in the Bunny dashboard"
        })
    );
    // C3: The secret key must never be echoed in the output
    assert!(!text(&r).contains("super-secret-custom-key"));
}

// covers: TOOL-PZ-07
#[tokio::test]
async fn pullzone_reset_security_key_omitted_key() {
    let (mock, srv) = setup().await;

    Mock::given(method("POST"))
        .and(path("/pullzone/123/resetSecurityKey"))
        .and(header("AccessKey", TEST_KEY))
        .and(body_json(json!({})))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({})))
        .expect(1)
        .mount(&mock)
        .await;

    let args = PullZoneResetSecurityKeyArgs {
        id: 123,
        security_key: None,
    };
    let r = srv
        .bunny_pullzone_reset_security_key(Parameters(args))
        .await
        .unwrap();
    assert_ok(&r);
    let val = json(&r);
    assert_eq!(
        val,
        json!({
            "ok": true,
            "id": 123,
            "note": "new key is visible in the Bunny dashboard"
        })
    );
}

// covers: TOOL-PZ-07
#[tokio::test]
async fn pullzone_reset_security_key_api_error() {
    let (mock, srv) = setup().await;

    Mock::given(method("POST"))
        .and(path("/pullzone/123/resetSecurityKey"))
        .and(header("AccessKey", TEST_KEY))
        .respond_with(ResponseTemplate::new(500))
        .expect(1)
        .mount(&mock)
        .await;

    let args = PullZoneResetSecurityKeyArgs {
        id: 123,
        security_key: None,
    };
    let r = srv
        .bunny_pullzone_reset_security_key(Parameters(args))
        .await
        .unwrap();
    assert_err(&r, "500");
}

// covers: TOOL-PZ-07
#[tokio::test]
async fn pullzone_reset_security_key_invalid_args() {
    let (_, srv) = setup().await;

    let args_zero_id = PullZoneResetSecurityKeyArgs {
        id: 0,
        security_key: None,
    };
    let r = srv
        .bunny_pullzone_reset_security_key(Parameters(args_zero_id))
        .await
        .unwrap();
    assert_err(&r, "id must be >= 1");

    let args_empty_key = PullZoneResetSecurityKeyArgs {
        id: 123,
        security_key: Some("   ".to_string()),
    };
    let r = srv
        .bunny_pullzone_reset_security_key(Parameters(args_empty_key))
        .await
        .unwrap();
    assert_err(&r, "security_key cannot be empty");
}
