//! Integration tests for Access List tools (specs/tools/access.md, REQ-PZ-08).

mod common;

use bunnycdn_mcp::tools::access::{AccessAction, AccessList, PullZoneAccessListArgs};
use rmcp::handler::server::wrapper::Parameters;
use serde_json::json;
use wiremock::matchers::{body_json, header, method, path};
use wiremock::{Mock, ResponseTemplate};

use common::{TEST_KEY, assert_err, assert_ok, json as parse_json, setup};

// covers: TOOL-AC-01, REQ-PZ-08
#[tokio::test]
async fn access_list_allowed_referrer_add_success() {
    let (mock, srv) = setup().await;

    Mock::given(method("POST"))
        .and(path("/pullzone/10/addAllowedReferrer"))
        .and(header("AccessKey", TEST_KEY))
        .and(body_json(json!({ "Hostname": "cdn.example.com" })))
        .respond_with(ResponseTemplate::new(204))
        .expect(1)
        .mount(&mock)
        .await;

    let res = srv
        .bunny_pullzone_access_list(Parameters(PullZoneAccessListArgs {
            id: 10,
            list: AccessList::AllowedReferrer,
            action: AccessAction::Add,
            value: "cdn.example.com".to_string(),
        }))
        .await
        .unwrap();

    assert_ok(&res);
    let out = parse_json(&res);
    assert_eq!(out["ok"], true);
    assert_eq!(out["id"], 10);
    assert_eq!(out["list"], "allowed_referrer");
    assert_eq!(out["action"], "add");
    assert_eq!(out["value"], "cdn.example.com");
}

// covers: TOOL-AC-01, REQ-PZ-08
#[tokio::test]
async fn access_list_allowed_referrer_remove_success() {
    let (mock, srv) = setup().await;

    Mock::given(method("POST"))
        .and(path("/pullzone/10/removeAllowedReferrer"))
        .and(header("AccessKey", TEST_KEY))
        .and(body_json(json!({ "Hostname": "cdn.example.com" })))
        .respond_with(ResponseTemplate::new(204))
        .expect(1)
        .mount(&mock)
        .await;

    let res = srv
        .bunny_pullzone_access_list(Parameters(PullZoneAccessListArgs {
            id: 10,
            list: AccessList::AllowedReferrer,
            action: AccessAction::Remove,
            value: "cdn.example.com".to_string(),
        }))
        .await
        .unwrap();

    assert_ok(&res);
    let out = parse_json(&res);
    assert_eq!(out["ok"], true);
    assert_eq!(out["id"], 10);
    assert_eq!(out["list"], "allowed_referrer");
    assert_eq!(out["action"], "remove");
    assert_eq!(out["value"], "cdn.example.com");
}

// covers: TOOL-AC-01, REQ-PZ-08
#[tokio::test]
async fn access_list_blocked_referrer_add_success() {
    let (mock, srv) = setup().await;

    Mock::given(method("POST"))
        .and(path("/pullzone/20/addBlockedReferrer"))
        .and(header("AccessKey", TEST_KEY))
        .and(body_json(json!({ "Hostname": "bad.example.com" })))
        .respond_with(ResponseTemplate::new(204))
        .expect(1)
        .mount(&mock)
        .await;

    let res = srv
        .bunny_pullzone_access_list(Parameters(PullZoneAccessListArgs {
            id: 20,
            list: AccessList::BlockedReferrer,
            action: AccessAction::Add,
            value: "bad.example.com".to_string(),
        }))
        .await
        .unwrap();

    assert_ok(&res);
    let out = parse_json(&res);
    assert_eq!(out["ok"], true);
    assert_eq!(out["id"], 20);
    assert_eq!(out["list"], "blocked_referrer");
    assert_eq!(out["action"], "add");
    assert_eq!(out["value"], "bad.example.com");
}

// covers: TOOL-AC-01, REQ-PZ-08
#[tokio::test]
async fn access_list_blocked_referrer_remove_success() {
    let (mock, srv) = setup().await;

    Mock::given(method("POST"))
        .and(path("/pullzone/20/removeBlockedReferrer"))
        .and(header("AccessKey", TEST_KEY))
        .and(body_json(json!({ "Hostname": "bad.example.com" })))
        .respond_with(ResponseTemplate::new(204))
        .expect(1)
        .mount(&mock)
        .await;

    let res = srv
        .bunny_pullzone_access_list(Parameters(PullZoneAccessListArgs {
            id: 20,
            list: AccessList::BlockedReferrer,
            action: AccessAction::Remove,
            value: "bad.example.com".to_string(),
        }))
        .await
        .unwrap();

    assert_ok(&res);
    let out = parse_json(&res);
    assert_eq!(out["ok"], true);
    assert_eq!(out["id"], 20);
    assert_eq!(out["list"], "blocked_referrer");
    assert_eq!(out["action"], "remove");
    assert_eq!(out["value"], "bad.example.com");
}

// covers: TOOL-AC-01, REQ-PZ-08
#[tokio::test]
async fn access_list_blocked_ip_add_success() {
    let (mock, srv) = setup().await;

    Mock::given(method("POST"))
        .and(path("/pullzone/30/addBlockedIp"))
        .and(header("AccessKey", TEST_KEY))
        .and(body_json(json!({ "BlockedIp": "192.168.1.1" })))
        .respond_with(ResponseTemplate::new(204))
        .expect(1)
        .mount(&mock)
        .await;

    let res = srv
        .bunny_pullzone_access_list(Parameters(PullZoneAccessListArgs {
            id: 30,
            list: AccessList::BlockedIp,
            action: AccessAction::Add,
            value: "192.168.1.1".to_string(),
        }))
        .await
        .unwrap();

    assert_ok(&res);
    let out = parse_json(&res);
    assert_eq!(out["ok"], true);
    assert_eq!(out["id"], 30);
    assert_eq!(out["list"], "blocked_ip");
    assert_eq!(out["action"], "add");
    assert_eq!(out["value"], "192.168.1.1");
}

// covers: TOOL-AC-01, REQ-PZ-08
#[tokio::test]
async fn access_list_blocked_ip_remove_success() {
    let (mock, srv) = setup().await;

    Mock::given(method("POST"))
        .and(path("/pullzone/30/removeBlockedIp"))
        .and(header("AccessKey", TEST_KEY))
        .and(body_json(json!({ "BlockedIp": "192.168.1.1" })))
        .respond_with(ResponseTemplate::new(204))
        .expect(1)
        .mount(&mock)
        .await;

    let res = srv
        .bunny_pullzone_access_list(Parameters(PullZoneAccessListArgs {
            id: 30,
            list: AccessList::BlockedIp,
            action: AccessAction::Remove,
            value: "192.168.1.1".to_string(),
        }))
        .await
        .unwrap();

    assert_ok(&res);
    let out = parse_json(&res);
    assert_eq!(out["ok"], true);
    assert_eq!(out["id"], 30);
    assert_eq!(out["list"], "blocked_ip");
    assert_eq!(out["action"], "remove");
    assert_eq!(out["value"], "192.168.1.1");
}

// covers: TOOL-AC-01, REQ-PZ-08
#[tokio::test]
async fn access_list_blocked_ip_cidr_success() {
    let (mock, srv) = setup().await;

    Mock::given(method("POST"))
        .and(path("/pullzone/30/addBlockedIp"))
        .and(header("AccessKey", TEST_KEY))
        .and(body_json(json!({ "BlockedIp": "10.0.0.0/24" })))
        .respond_with(ResponseTemplate::new(204))
        .expect(1)
        .mount(&mock)
        .await;

    let res = srv
        .bunny_pullzone_access_list(Parameters(PullZoneAccessListArgs {
            id: 30,
            list: AccessList::BlockedIp,
            action: AccessAction::Add,
            value: " 10.0.0.0/24 ".to_string(),
        }))
        .await
        .unwrap();

    assert_ok(&res);
    let out = parse_json(&res);
    assert_eq!(out["ok"], true);
    assert_eq!(out["id"], 30);
    assert_eq!(out["list"], "blocked_ip");
    assert_eq!(out["action"], "add");
    assert_eq!(out["value"], "10.0.0.0/24");
}

// covers: TOOL-AC-01, REQ-PZ-08
#[tokio::test]
async fn access_list_referrer_normalized_with_scheme_and_path() {
    let (mock, srv) = setup().await;

    Mock::given(method("POST"))
        .and(path("/pullzone/40/addAllowedReferrer"))
        .and(header("AccessKey", TEST_KEY))
        .and(body_json(json!({ "Hostname": "cdn.example.com" })))
        .respond_with(ResponseTemplate::new(204))
        .expect(1)
        .mount(&mock)
        .await;

    let res = srv
        .bunny_pullzone_access_list(Parameters(PullZoneAccessListArgs {
            id: 40,
            list: AccessList::AllowedReferrer,
            action: AccessAction::Add,
            value: "  https://CDN.EXAMPLE.COM:443/some/path?foo=1#section  ".to_string(),
        }))
        .await
        .unwrap();

    assert_ok(&res);
    let out = parse_json(&res);
    assert_eq!(out["ok"], true);
    assert_eq!(out["id"], 40);
    assert_eq!(out["list"], "allowed_referrer");
    assert_eq!(out["action"], "add");
    assert_eq!(out["value"], "cdn.example.com");
}

// covers: TOOL-AC-01, REQ-PZ-08
#[tokio::test]
async fn access_list_validation_failures() {
    let (_mock, srv) = setup().await;

    // id == 0
    let res = srv
        .bunny_pullzone_access_list(Parameters(PullZoneAccessListArgs {
            id: 0,
            list: AccessList::BlockedIp,
            action: AccessAction::Add,
            value: "1.2.3.4".to_string(),
        }))
        .await
        .unwrap();
    assert_err(&res, "id must be >= 1");

    // empty value
    let res = srv
        .bunny_pullzone_access_list(Parameters(PullZoneAccessListArgs {
            id: 1,
            list: AccessList::BlockedIp,
            action: AccessAction::Add,
            value: "   ".to_string(),
        }))
        .await
        .unwrap();
    assert_err(&res, "value cannot be empty");

    // invalid IP
    let res = srv
        .bunny_pullzone_access_list(Parameters(PullZoneAccessListArgs {
            id: 1,
            list: AccessList::BlockedIp,
            action: AccessAction::Add,
            value: "not-an-ip".to_string(),
        }))
        .await
        .unwrap();
    assert_err(&res, "Invalid argument");

    // invalid CIDR prefix
    let res = srv
        .bunny_pullzone_access_list(Parameters(PullZoneAccessListArgs {
            id: 1,
            list: AccessList::BlockedIp,
            action: AccessAction::Add,
            value: "1.2.3.4/33".to_string(),
        }))
        .await
        .unwrap();
    assert_err(&res, "Invalid argument");

    // referrer without scheme but containing a path
    let res = srv
        .bunny_pullzone_access_list(Parameters(PullZoneAccessListArgs {
            id: 1,
            list: AccessList::AllowedReferrer,
            action: AccessAction::Add,
            value: "example.com/some/path".to_string(),
        }))
        .await
        .unwrap();
    assert_err(&res, "referrer must not contain a path");
}

// covers: TOOL-AC-01, REQ-PZ-08
#[tokio::test]
async fn access_list_api_error() {
    let (mock, srv) = setup().await;

    Mock::given(method("POST"))
        .and(path("/pullzone/999/addBlockedIp"))
        .respond_with(ResponseTemplate::new(404))
        .expect(1)
        .mount(&mock)
        .await;

    let res = srv
        .bunny_pullzone_access_list(Parameters(PullZoneAccessListArgs {
            id: 999,
            list: AccessList::BlockedIp,
            action: AccessAction::Add,
            value: "1.2.3.4".to_string(),
        }))
        .await
        .unwrap();

    assert_err(&res, "Not found (404)");
}
