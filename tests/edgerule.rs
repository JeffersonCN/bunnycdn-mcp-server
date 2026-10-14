//! Integration tests for Edge Rule tools (specs/tools/edgerule.md, REQ-ER-01..04).

mod common;

use bunnycdn_mcp::tools::edgerule::{
    EdgeRuleActionInput, EdgeRuleDeleteArgs, EdgeRuleListArgs, EdgeRuleSetEnabledArgs,
    EdgeRuleTriggerInput, EdgeRuleUpsertArgs,
};
use common::{TEST_KEY, assert_err, assert_ok, fixture, json, setup};
use rmcp::handler::server::wrapper::Parameters;
use serde_json::json;
use wiremock::matchers::{body_json, header, method, path};
use wiremock::{Mock, ResponseTemplate};

// covers: TOOL-ER-01
#[tokio::test]
async fn edgerule_list_success() {
    let (mock, srv) = setup().await;

    Mock::given(method("GET"))
        .and(path("/pullzone/123"))
        .and(header("AccessKey", TEST_KEY))
        .and(header("Accept", "application/json"))
        .respond_with(ResponseTemplate::new(200).set_body_json(fixture("edgerule_pullzone")))
        .expect(1)
        .mount(&mock)
        .await;

    let res = srv
        .bunny_edgerule_list(Parameters(EdgeRuleListArgs { pullzone_id: 123 }))
        .await
        .unwrap();

    assert_ok(&res);
    let v = json(&res);
    assert_eq!(v["pullzone_id"], 123);

    let rules = v["rules"].as_array().expect("rules is array");
    assert_eq!(rules.len(), 3);

    // Sorted by OrderIndex
    assert_eq!(rules[0]["guid"], "3fa85f64-5717-4562-b3fc-2c963f66afa6");
    assert_eq!(rules[0]["description"], "Force SSL on all HTTP requests");
    assert_eq!(rules[0]["enabled"], true);
    assert_eq!(rules[0]["action"]["type"], "ForceSSL");
    assert!(rules[0]["action"].get("p1").is_none());
    assert!(rules[0].get("extra_actions").is_none());
    assert_eq!(rules[0]["match"], "MatchAny");
    assert_eq!(rules[0]["triggers"][0]["type"], "Url");
    assert_eq!(rules[0]["triggers"][0]["patterns"][0], "http://*");

    assert_eq!(rules[1]["guid"], "7b2d1c84-9123-4e56-8a90-1b2c3d4e5f67");
    assert_eq!(rules[1]["action"]["type"], "OverrideCacheTime");
    assert_eq!(rules[1]["action"]["p1"], "86400");
    assert_eq!(rules[1]["extra_actions"][0]["type"], "SetResponseHeader");
    assert_eq!(rules[1]["extra_actions"][0]["p1"], "X-Cache-Rule");
    assert_eq!(rules[1]["extra_actions"][0]["p2"], "img");

    assert_eq!(rules[2]["guid"], "a1b2c3d4-e5f6-7890-1234-56789abcdef0");
    assert_eq!(rules[2]["action"]["type"], "SetResponseHeader");
    assert_eq!(rules[2]["triggers"][0]["type"], "RequestHeader");
    assert_eq!(rules[2]["triggers"][0]["match"], "MatchAll");
    assert_eq!(rules[2]["triggers"][0]["parameter1"], "Authorization");
}

// covers: TOOL-ER-01
#[tokio::test]
async fn edgerule_list_empty_and_null_rules() {
    let (mock, srv) = setup().await;

    Mock::given(method("GET"))
        .and(path("/pullzone/456"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "Id": 456,
            "EdgeRules": []
        })))
        .mount(&mock)
        .await;

    let res = srv
        .bunny_edgerule_list(Parameters(EdgeRuleListArgs { pullzone_id: 456 }))
        .await
        .unwrap();
    assert_ok(&res);
    assert_eq!(json(&res)["rules"], json!([]));

    Mock::given(method("GET"))
        .and(path("/pullzone/789"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "Id": 789,
            "EdgeRules": null
        })))
        .mount(&mock)
        .await;

    let res = srv
        .bunny_edgerule_list(Parameters(EdgeRuleListArgs { pullzone_id: 789 }))
        .await
        .unwrap();
    assert_ok(&res);
    assert_eq!(json(&res)["rules"], json!([]));
}

// covers: TOOL-ER-01
#[tokio::test]
async fn edgerule_list_invalid_pullzone_id() {
    let (_mock, srv) = setup().await;

    let res = srv
        .bunny_edgerule_list(Parameters(EdgeRuleListArgs { pullzone_id: 0 }))
        .await
        .unwrap();
    assert_err(&res, "pullzone_id must be >= 1");
}

// covers: TOOL-ER-01
#[tokio::test]
async fn edgerule_list_not_found() {
    let (mock, srv) = setup().await;

    Mock::given(method("GET"))
        .and(path("/pullzone/999"))
        .respond_with(ResponseTemplate::new(404))
        .mount(&mock)
        .await;

    let res = srv
        .bunny_edgerule_list(Parameters(EdgeRuleListArgs { pullzone_id: 999 }))
        .await
        .unwrap();
    assert_err(&res, "Not found (404)");
}

// covers: TOOL-ER-02
#[tokio::test]
async fn edgerule_upsert_create_success() {
    let (mock, srv) = setup().await;

    Mock::given(method("POST"))
        .and(path("/pullzone/123/edgerules/addOrUpdate"))
        .and(header("AccessKey", TEST_KEY))
        .and(body_json(json!({
            "ActionType": 0,
            "ActionParameter1": null,
            "ActionParameter2": null,
            "ActionParameter3": null,
            "Triggers": [
                {
                    "Type": 0,
                    "PatternMatches": ["http://*"],
                    "PatternMatchingType": 0,
                    "Parameter1": null
                }
            ],
            "ExtraActions": [],
            "TriggerMatchingType": 0,
            "Description": "Force SSL on all HTTP requests",
            "Enabled": true
        })))
        .respond_with(ResponseTemplate::new(201))
        .expect(1)
        .mount(&mock)
        .await;

    Mock::given(method("GET"))
        .and(path("/pullzone/123"))
        .respond_with(ResponseTemplate::new(200).set_body_json(fixture("edgerule_pullzone")))
        .expect(1)
        .mount(&mock)
        .await;

    let res = srv
        .bunny_edgerule_upsert(Parameters(EdgeRuleUpsertArgs {
            pullzone_id: 123,
            guid: None,
            action_type: "forcessl".to_string(), // case-insensitive check
            action_parameter1: None,
            action_parameter2: None,
            action_parameter3: None,
            extra_actions: None,
            triggers: vec![EdgeRuleTriggerInput {
                r#type: "url".to_string(),
                patterns: vec!["http://*".to_string()],
                r#match: Some("matchany".to_string()),
                parameter1: None,
            }],
            r#match: Some("MatchAny".to_string()),
            description: Some("Force SSL on all HTTP requests".to_string()),
            enabled: Some(true),
        }))
        .await
        .unwrap();

    assert_ok(&res);
    let v = json(&res);
    assert_eq!(v["ok"], true);
    assert_eq!(v["created"], true);
    assert!(v.get("updated").is_none());
    assert_eq!(v["rule"]["guid"], "3fa85f64-5717-4562-b3fc-2c963f66afa6");
    assert_eq!(v["rule"]["action"]["type"], "ForceSSL");
}

// covers: TOOL-ER-02
#[tokio::test]
async fn edgerule_upsert_update_success() {
    let (mock, srv) = setup().await;

    Mock::given(method("POST"))
        .and(path("/pullzone/123/edgerules/addOrUpdate"))
        .and(header("AccessKey", TEST_KEY))
        .and(body_json(json!({
            "Guid": "7b2d1c84-9123-4e56-8a90-1b2c3d4e5f67",
            "ActionType": 3,
            "ActionParameter1": "86400",
            "ActionParameter2": null,
            "ActionParameter3": null,
            "Triggers": [
                {
                    "Type": 3,
                    "PatternMatches": ["jpg", "png", "webp", "css", "js"],
                    "PatternMatchingType": 0,
                    "Parameter1": null
                }
            ],
            "ExtraActions": [
                {
                    "ActionType": 5,
                    "ActionParameter1": "X-Cache-Rule",
                    "ActionParameter2": "img",
                    "ActionParameter3": null
                }
            ],
            "TriggerMatchingType": 0,
            "Description": "Cache static assets for 1 day",
            "Enabled": true
        })))
        .respond_with(ResponseTemplate::new(201))
        .expect(1)
        .mount(&mock)
        .await;

    Mock::given(method("GET"))
        .and(path("/pullzone/123"))
        .respond_with(ResponseTemplate::new(200).set_body_json(fixture("edgerule_pullzone")))
        .expect(1)
        .mount(&mock)
        .await;

    let res = srv
        .bunny_edgerule_upsert(Parameters(EdgeRuleUpsertArgs {
            pullzone_id: 123,
            guid: Some("7b2d1c84-9123-4e56-8a90-1b2c3d4e5f67".to_string()),
            action_type: "OverrideCacheTime".to_string(),
            action_parameter1: Some("86400".to_string()),
            action_parameter2: None,
            action_parameter3: None,
            extra_actions: Some(vec![EdgeRuleActionInput {
                r#type: "SetResponseHeader".to_string(),
                p1: Some("X-Cache-Rule".to_string()),
                p2: Some("img".to_string()),
                p3: None,
            }]),
            triggers: vec![EdgeRuleTriggerInput {
                r#type: "UrlExtension".to_string(),
                patterns: vec![
                    "jpg".into(),
                    "png".into(),
                    "webp".into(),
                    "css".into(),
                    "js".into(),
                ],
                r#match: Some("MatchAny".to_string()),
                parameter1: None,
            }],
            r#match: None,
            description: Some("Cache static assets for 1 day".to_string()),
            enabled: Some(true),
        }))
        .await
        .unwrap();

    assert_ok(&res);
    let v = json(&res);
    assert_eq!(v["ok"], true);
    assert_eq!(v["updated"], true);
    assert!(v.get("created").is_none());
    assert_eq!(v["rule"]["guid"], "7b2d1c84-9123-4e56-8a90-1b2c3d4e5f67");
    assert_eq!(v["rule"]["action"]["type"], "OverrideCacheTime");
    assert_eq!(v["rule"]["action"]["p1"], "86400");
    assert_eq!(v["rule"]["extra_actions"][0]["type"], "SetResponseHeader");
}

// covers: TOOL-ER-02
#[tokio::test]
async fn edgerule_upsert_validation_failures() {
    let (_mock, srv) = setup().await;

    // Invalid pullzone_id
    let res = srv
        .bunny_edgerule_upsert(Parameters(EdgeRuleUpsertArgs {
            pullzone_id: 0,
            guid: None,
            action_type: "ForceSSL".to_string(),
            action_parameter1: None,
            action_parameter2: None,
            action_parameter3: None,
            extra_actions: None,
            triggers: vec![EdgeRuleTriggerInput {
                r#type: "Url".to_string(),
                patterns: vec!["*".into()],
                r#match: None,
                parameter1: None,
            }],
            r#match: None,
            description: None,
            enabled: None,
        }))
        .await
        .unwrap();
    assert_err(&res, "pullzone_id must be >= 1");

    // Invalid action_type: lists all valid enum values
    let res = srv
        .bunny_edgerule_upsert(Parameters(EdgeRuleUpsertArgs {
            pullzone_id: 123,
            guid: None,
            action_type: "UnknownInvalidAction".to_string(),
            action_parameter1: None,
            action_parameter2: None,
            action_parameter3: None,
            extra_actions: None,
            triggers: vec![EdgeRuleTriggerInput {
                r#type: "Url".to_string(),
                patterns: vec!["*".into()],
                r#match: None,
                parameter1: None,
            }],
            r#match: None,
            description: None,
            enabled: None,
        }))
        .await
        .unwrap();
    assert_err(&res, "invalid action_type");
    assert_err(&res, "ForceSSL");
    assert_err(&res, "OverrideCacheTime");
    assert_err(&res, "StripCookiesClientToOrigin");

    // Invalid extra_actions type
    let res = srv
        .bunny_edgerule_upsert(Parameters(EdgeRuleUpsertArgs {
            pullzone_id: 123,
            guid: None,
            action_type: "ForceSSL".to_string(),
            action_parameter1: None,
            action_parameter2: None,
            action_parameter3: None,
            extra_actions: Some(vec![EdgeRuleActionInput {
                r#type: "FakeExtraAction".to_string(),
                p1: None,
                p2: None,
                p3: None,
            }]),
            triggers: vec![EdgeRuleTriggerInput {
                r#type: "Url".to_string(),
                patterns: vec!["*".into()],
                r#match: None,
                parameter1: None,
            }],
            r#match: None,
            description: None,
            enabled: None,
        }))
        .await
        .unwrap();
    assert_err(&res, "invalid extra_action type");

    // Empty triggers
    let res = srv
        .bunny_edgerule_upsert(Parameters(EdgeRuleUpsertArgs {
            pullzone_id: 123,
            guid: None,
            action_type: "ForceSSL".to_string(),
            action_parameter1: None,
            action_parameter2: None,
            action_parameter3: None,
            extra_actions: None,
            triggers: vec![],
            r#match: None,
            description: None,
            enabled: None,
        }))
        .await
        .unwrap();
    assert_err(&res, "triggers must contain between 1 and 5 items");

    // More than 5 triggers
    let res = srv
        .bunny_edgerule_upsert(Parameters(EdgeRuleUpsertArgs {
            pullzone_id: 123,
            guid: None,
            action_type: "ForceSSL".to_string(),
            action_parameter1: None,
            action_parameter2: None,
            action_parameter3: None,
            extra_actions: None,
            triggers: (0..6)
                .map(|_| EdgeRuleTriggerInput {
                    r#type: "Url".to_string(),
                    patterns: vec!["*".into()],
                    r#match: None,
                    parameter1: None,
                })
                .collect(),
            r#match: None,
            description: None,
            enabled: None,
        }))
        .await
        .unwrap();
    assert_err(&res, "triggers must contain between 1 and 5 items");

    // Empty trigger pattern
    let res = srv
        .bunny_edgerule_upsert(Parameters(EdgeRuleUpsertArgs {
            pullzone_id: 123,
            guid: None,
            action_type: "ForceSSL".to_string(),
            action_parameter1: None,
            action_parameter2: None,
            action_parameter3: None,
            extra_actions: None,
            triggers: vec![EdgeRuleTriggerInput {
                r#type: "Url".to_string(),
                patterns: vec![],
                r#match: None,
                parameter1: None,
            }],
            r#match: None,
            description: None,
            enabled: None,
        }))
        .await
        .unwrap();
    assert_err(&res, "trigger patterns must not be empty");

    // Invalid trigger type
    let res = srv
        .bunny_edgerule_upsert(Parameters(EdgeRuleUpsertArgs {
            pullzone_id: 123,
            guid: None,
            action_type: "ForceSSL".to_string(),
            action_parameter1: None,
            action_parameter2: None,
            action_parameter3: None,
            extra_actions: None,
            triggers: vec![EdgeRuleTriggerInput {
                r#type: "BadTriggerType".to_string(),
                patterns: vec!["*".into()],
                r#match: None,
                parameter1: None,
            }],
            r#match: None,
            description: None,
            enabled: None,
        }))
        .await
        .unwrap();
    assert_err(&res, "invalid trigger type");
    assert_err(&res, "RequestHeader");

    // Invalid pattern matching type
    let res = srv
        .bunny_edgerule_upsert(Parameters(EdgeRuleUpsertArgs {
            pullzone_id: 123,
            guid: None,
            action_type: "ForceSSL".to_string(),
            action_parameter1: None,
            action_parameter2: None,
            action_parameter3: None,
            extra_actions: None,
            triggers: vec![EdgeRuleTriggerInput {
                r#type: "Url".to_string(),
                patterns: vec!["*".into()],
                r#match: Some("InvalidPatternMatch".to_string()),
                parameter1: None,
            }],
            r#match: None,
            description: None,
            enabled: None,
        }))
        .await
        .unwrap();
    assert_err(&res, "invalid pattern match type");

    // Invalid top-level trigger match
    let res = srv
        .bunny_edgerule_upsert(Parameters(EdgeRuleUpsertArgs {
            pullzone_id: 123,
            guid: None,
            action_type: "ForceSSL".to_string(),
            action_parameter1: None,
            action_parameter2: None,
            action_parameter3: None,
            extra_actions: None,
            triggers: vec![EdgeRuleTriggerInput {
                r#type: "Url".to_string(),
                patterns: vec!["*".into()],
                r#match: None,
                parameter1: None,
            }],
            r#match: Some("InvalidTopMatch".to_string()),
            description: None,
            enabled: None,
        }))
        .await
        .unwrap();
    assert_err(&res, "invalid match type");

    // Empty whitespace guid
    let res = srv
        .bunny_edgerule_upsert(Parameters(EdgeRuleUpsertArgs {
            pullzone_id: 123,
            guid: Some("   ".to_string()),
            action_type: "ForceSSL".to_string(),
            action_parameter1: None,
            action_parameter2: None,
            action_parameter3: None,
            extra_actions: None,
            triggers: vec![EdgeRuleTriggerInput {
                r#type: "Url".to_string(),
                patterns: vec!["*".into()],
                r#match: None,
                parameter1: None,
            }],
            r#match: None,
            description: None,
            enabled: None,
        }))
        .await
        .unwrap();
    assert_err(&res, "guid cannot be empty");
}

// covers: TOOL-ER-02
#[tokio::test]
async fn edgerule_upsert_api_error() {
    let (mock, srv) = setup().await;

    Mock::given(method("POST"))
        .and(path("/pullzone/123/edgerules/addOrUpdate"))
        .respond_with(ResponseTemplate::new(400).set_body_json(json!({
            "ErrorKey": "rule.invalid",
            "Field": "Triggers",
            "Message": "Pattern match conflict"
        })))
        .mount(&mock)
        .await;

    let res = srv
        .bunny_edgerule_upsert(Parameters(EdgeRuleUpsertArgs {
            pullzone_id: 123,
            guid: None,
            action_type: "ForceSSL".to_string(),
            action_parameter1: None,
            action_parameter2: None,
            action_parameter3: None,
            extra_actions: None,
            triggers: vec![EdgeRuleTriggerInput {
                r#type: "Url".to_string(),
                patterns: vec!["*".into()],
                r#match: None,
                parameter1: None,
            }],
            r#match: None,
            description: None,
            enabled: None,
        }))
        .await
        .unwrap();

    assert_err(&res, "Bunny API error 400 (rule.invalid)");
    assert_err(&res, "field=Triggers");
    assert_err(&res, "Pattern match conflict");
}

// covers: TOOL-ER-03
#[tokio::test]
async fn edgerule_set_enabled_success() {
    let (mock, srv) = setup().await;

    Mock::given(method("POST"))
        .and(path(
            "/pullzone/123/edgerules/3fa85f64-5717-4562-b3fc-2c963f66afa6/setEdgeRuleEnabled",
        ))
        .and(header("AccessKey", TEST_KEY))
        .and(body_json(json!({
            "Id": 123,
            "Value": false
        })))
        .respond_with(ResponseTemplate::new(204))
        .expect(1)
        .mount(&mock)
        .await;

    let res = srv
        .bunny_edgerule_set_enabled(Parameters(EdgeRuleSetEnabledArgs {
            pullzone_id: 123,
            guid: "3fa85f64-5717-4562-b3fc-2c963f66afa6".to_string(),
            enabled: false,
        }))
        .await
        .unwrap();

    assert_ok(&res);
    let v = json(&res);
    assert_eq!(v["ok"], true);
    assert_eq!(v["pullzone_id"], 123);
    assert_eq!(v["guid"], "3fa85f64-5717-4562-b3fc-2c963f66afa6");
    assert_eq!(v["enabled"], false);
}

// covers: TOOL-ER-03
#[tokio::test]
async fn edgerule_set_enabled_percent_encodes_guid() {
    let (mock, srv) = setup().await;

    Mock::given(method("POST"))
        .and(path(
            "/pullzone/123/edgerules/custom%2Fguid%40tag/setEdgeRuleEnabled",
        ))
        .and(body_json(json!({
            "Id": 123,
            "Value": true
        })))
        .respond_with(ResponseTemplate::new(204))
        .expect(1)
        .mount(&mock)
        .await;

    let res = srv
        .bunny_edgerule_set_enabled(Parameters(EdgeRuleSetEnabledArgs {
            pullzone_id: 123,
            guid: "custom/guid@tag".to_string(),
            enabled: true,
        }))
        .await
        .unwrap();

    assert_ok(&res);
}

// covers: TOOL-ER-03
#[tokio::test]
async fn edgerule_set_enabled_validation_and_api_error() {
    let (mock, srv) = setup().await;

    let res = srv
        .bunny_edgerule_set_enabled(Parameters(EdgeRuleSetEnabledArgs {
            pullzone_id: 0,
            guid: "rule-1".to_string(),
            enabled: true,
        }))
        .await
        .unwrap();
    assert_err(&res, "pullzone_id must be >= 1");

    let res = srv
        .bunny_edgerule_set_enabled(Parameters(EdgeRuleSetEnabledArgs {
            pullzone_id: 123,
            guid: "   ".to_string(),
            enabled: true,
        }))
        .await
        .unwrap();
    assert_err(&res, "guid cannot be empty");

    Mock::given(method("POST"))
        .and(path("/pullzone/123/edgerules/rule-1/setEdgeRuleEnabled"))
        .respond_with(ResponseTemplate::new(404))
        .mount(&mock)
        .await;

    let res = srv
        .bunny_edgerule_set_enabled(Parameters(EdgeRuleSetEnabledArgs {
            pullzone_id: 123,
            guid: "rule-1".to_string(),
            enabled: true,
        }))
        .await
        .unwrap();
    assert_err(&res, "Not found (404)");
}

// covers: TOOL-ER-04
#[tokio::test]
async fn edgerule_delete_success() {
    let (mock, srv) = setup().await;

    Mock::given(method("DELETE"))
        .and(path(
            "/pullzone/123/edgerules/3fa85f64-5717-4562-b3fc-2c963f66afa6",
        ))
        .and(header("AccessKey", TEST_KEY))
        .respond_with(ResponseTemplate::new(204))
        .expect(1)
        .mount(&mock)
        .await;

    let res = srv
        .bunny_edgerule_delete(Parameters(EdgeRuleDeleteArgs {
            pullzone_id: 123,
            guid: "3fa85f64-5717-4562-b3fc-2c963f66afa6".to_string(),
        }))
        .await
        .unwrap();

    assert_ok(&res);
    let v = json(&res);
    assert_eq!(v["ok"], true);
    assert_eq!(v["pullzone_id"], 123);
    assert_eq!(v["deleted_guid"], "3fa85f64-5717-4562-b3fc-2c963f66afa6");
}

// covers: TOOL-ER-04
#[tokio::test]
async fn edgerule_delete_validation_and_api_error() {
    let (mock, srv) = setup().await;

    let res = srv
        .bunny_edgerule_delete(Parameters(EdgeRuleDeleteArgs {
            pullzone_id: 0,
            guid: "rule-1".to_string(),
        }))
        .await
        .unwrap();
    assert_err(&res, "pullzone_id must be >= 1");

    let res = srv
        .bunny_edgerule_delete(Parameters(EdgeRuleDeleteArgs {
            pullzone_id: 123,
            guid: "".to_string(),
        }))
        .await
        .unwrap();
    assert_err(&res, "guid cannot be empty");

    Mock::given(method("DELETE"))
        .and(path("/pullzone/123/edgerules/rule-1"))
        .respond_with(ResponseTemplate::new(404))
        .mount(&mock)
        .await;

    let res = srv
        .bunny_edgerule_delete(Parameters(EdgeRuleDeleteArgs {
            pullzone_id: 123,
            guid: "rule-1".to_string(),
        }))
        .await
        .unwrap();
    assert_err(&res, "Not found (404)");
}
