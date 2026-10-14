//! Wiremock tests for statistics tools: `bunny_statistics` and `bunny_pullzone_feature_statistics`
//! (design §10, specs/tools/stats.md, C6, REQ-ST-01..03).

mod common;

use bunnycdn_mcp::model::stats::FeatureKind;
use bunnycdn_mcp::tools::stats::{PullzoneFeatureStatisticsArgs, StatisticsArgs};
use rmcp::handler::server::wrapper::Parameters;
use serde_json::json;
use wiremock::matchers::{header, method, path, query_param};
use wiremock::{Mock, ResponseTemplate};

// covers: TOOL-ST-01
#[tokio::test]
async fn statistics_summary_matches_snapshot() {
    let (mock, srv) = common::setup().await;

    Mock::given(method("GET"))
        .and(path("/statistics"))
        .and(header("AccessKey", common::TEST_KEY))
        .and(query_param("dateFrom", "2026-09-14T00:00:00Z"))
        .and(query_param("dateTo", "2026-10-14T23:59:59Z"))
        .and(query_param("pullZone", "123"))
        .and(query_param("hourly", "false"))
        .and(query_param("loadErrors", "true"))
        .and(query_param("loadOriginResponseTimes", "true"))
        .and(query_param("loadGeographicTrafficDistribution", "true"))
        .respond_with(ResponseTemplate::new(200).set_body_json(common::fixture("stats_summary")))
        .mount(&mock)
        .await;

    let args = StatisticsArgs {
        pullzone_id: Some(123),
        date_from: Some("2026-09-14".to_string()),
        date_to: Some("2026-10-14".to_string()),
        hourly: Some(false),
        include_series: Some(true),
    };

    let res = srv.bunny_statistics(Parameters(args)).await.unwrap();
    common::assert_ok(&res);
    let val = common::json(&res);

    insta::assert_json_snapshot!(val);
}

// covers: TOOL-ST-01
#[tokio::test]
async fn statistics_account_wide_omits_pullzone_and_series() {
    let (mock, srv) = common::setup().await;

    Mock::given(method("GET"))
        .and(path("/statistics"))
        .and(header("AccessKey", common::TEST_KEY))
        .and(query_param("dateFrom", "2026-09-14T00:00:00Z"))
        .and(query_param("dateTo", "2026-10-14T23:59:59Z"))
        .and(query_param("hourly", "false"))
        .and(query_param("loadErrors", "true"))
        .and(query_param("loadOriginResponseTimes", "true"))
        .and(query_param("loadGeographicTrafficDistribution", "true"))
        .respond_with(ResponseTemplate::new(200).set_body_json(common::fixture("stats_summary")))
        .mount(&mock)
        .await;

    let args = StatisticsArgs {
        pullzone_id: None,
        date_from: Some("2026-09-14".to_string()),
        date_to: Some("2026-10-14".to_string()),
        hourly: None,
        include_series: None,
    };

    let res = srv.bunny_statistics(Parameters(args)).await.unwrap();
    common::assert_ok(&res);
    let val = common::json(&res);

    assert_eq!(val.get("pullzone_id"), None);
    assert_eq!(val.get("series"), None);
    assert_eq!(val.get("requests"), Some(&json!(45678)));
    assert_eq!(
        val.get("bandwidth"),
        Some(&json!({"bytes": 123456789, "human": "117.7 MiB"}))
    );
}

// covers: TOOL-ST-01
#[tokio::test]
async fn statistics_validation_errors() {
    let (_, srv) = common::setup().await;

    // pullzone_id == 0
    let res = srv
        .bunny_statistics(Parameters(StatisticsArgs {
            pullzone_id: Some(0),
            date_from: None,
            date_to: None,
            hourly: None,
            include_series: None,
        }))
        .await
        .unwrap();
    common::assert_err(&res, "pullzone_id must be >= 1");

    // Invalid date format
    let res = srv
        .bunny_statistics(Parameters(StatisticsArgs {
            pullzone_id: Some(1),
            date_from: Some("not-a-date".into()),
            date_to: None,
            hourly: None,
            include_series: None,
        }))
        .await
        .unwrap();
    common::assert_err(&res, "expected format YYYY-MM-DD");

    // Inverted range
    let res = srv
        .bunny_statistics(Parameters(StatisticsArgs {
            pullzone_id: Some(1),
            date_from: Some("2026-10-15".into()),
            date_to: Some("2026-10-14".into()),
            hourly: None,
            include_series: None,
        }))
        .await
        .unwrap();
    common::assert_err(&res, "date_from must be <= date_to");

    // Range > 366 days
    let res = srv
        .bunny_statistics(Parameters(StatisticsArgs {
            pullzone_id: Some(1),
            date_from: Some("2024-01-01".into()),
            date_to: Some("2025-01-05".into()),
            hourly: None,
            include_series: None,
        }))
        .await
        .unwrap();
    common::assert_err(&res, "date range must not exceed 366 days");
}

// covers: TOOL-ST-01
#[tokio::test]
async fn statistics_api_errors() {
    let (mock, srv) = common::setup().await;

    Mock::given(method("GET"))
        .and(path("/statistics"))
        .respond_with(ResponseTemplate::new(401))
        .mount(&mock)
        .await;

    let res = srv
        .bunny_statistics(Parameters(StatisticsArgs {
            pullzone_id: Some(123),
            date_from: Some("2026-09-14".into()),
            date_to: Some("2026-10-14".into()),
            hourly: None,
            include_series: None,
        }))
        .await
        .unwrap();
    common::assert_err(&res, "rejected the API key (401)");
}

// covers: TOOL-ST-02
#[tokio::test]
async fn pullzone_feature_statistics_optimizer_path_and_totals() {
    let (mock, srv) = common::setup().await;

    Mock::given(method("GET"))
        .and(path("/pullzone/123/optimizer/statistics"))
        .and(header("AccessKey", common::TEST_KEY))
        .and(query_param("dateFrom", "2026-09-14T00:00:00Z"))
        .and(query_param("dateTo", "2026-10-14T23:59:59Z"))
        .and(query_param("hourly", "false"))
        .respond_with(ResponseTemplate::new(200).set_body_json(common::fixture("stats_optimizer")))
        .mount(&mock)
        .await;

    let args = PullzoneFeatureStatisticsArgs {
        pullzone_id: 123,
        kind: FeatureKind::Optimizer,
        date_from: Some("2026-09-14".to_string()),
        date_to: Some("2026-10-14".to_string()),
        hourly: Some(false),
        include_series: Some(true),
    };

    let res = srv
        .bunny_pullzone_feature_statistics(Parameters(args))
        .await
        .unwrap();
    common::assert_ok(&res);
    let val = common::json(&res);

    assert_eq!(val.get("pullzone_id"), Some(&json!(123)));
    assert_eq!(val.get("kind"), Some(&json!("optimizer")));
    assert_eq!(
        val.get("range"),
        Some(&json!({"from": "2026-09-14", "to": "2026-10-14"}))
    );

    let totals = val.get("totals").expect("totals object");
    assert_eq!(totals.get("total_requests_optimized"), Some(&json!(1000.0)));
    assert_eq!(totals.get("total_traffic_saved"), Some(&json!(5000000.0)));
    assert_eq!(totals.get("average_processing_time"), Some(&json!(12.5)));
    assert_eq!(totals.get("average_compression_ratio"), Some(&json!(75.0)));
    assert_eq!(totals.get("requests_optimized_sum"), Some(&json!(1000)));
    assert_eq!(totals.get("traffic_saved_sum"), Some(&json!(5000000)));

    let series = val.get("series").expect("series object");
    assert!(series.get("requests_optimized").is_some());
    assert!(series.get("traffic_saved").is_some());
}

// covers: TOOL-ST-02
#[tokio::test]
async fn pullzone_feature_statistics_origin_shield_queue_path() {
    let (mock, srv) = common::setup().await;

    Mock::given(method("GET"))
        .and(path("/pullzone/123/originshield/queuestatistics"))
        .and(header("AccessKey", common::TEST_KEY))
        .and(query_param("dateFrom", "2026-09-14T00:00:00Z"))
        .and(query_param("dateTo", "2026-10-14T23:59:59Z"))
        .and(query_param("hourly", "true"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(common::fixture("stats_origin_shield")),
        )
        .mount(&mock)
        .await;

    let args = PullzoneFeatureStatisticsArgs {
        pullzone_id: 123,
        kind: FeatureKind::OriginShieldQueue,
        date_from: Some("2026-09-14".to_string()),
        date_to: Some("2026-10-14".to_string()),
        hourly: Some(true),
        include_series: Some(false),
    };

    let res = srv
        .bunny_pullzone_feature_statistics(Parameters(args))
        .await
        .unwrap();
    common::assert_ok(&res);
    let val = common::json(&res);

    assert_eq!(val.get("pullzone_id"), Some(&json!(123)));
    assert_eq!(val.get("kind"), Some(&json!("origin_shield_queue")));
    assert_eq!(val.get("series"), None);

    let totals = val.get("totals").expect("totals object");
    assert_eq!(totals.get("concurrent_requests_sum"), Some(&json!(120)));
    assert_eq!(totals.get("queued_requests_sum"), Some(&json!(15)));
}

// covers: TOOL-ST-02
#[tokio::test]
async fn pullzone_feature_statistics_safehop_path() {
    let (mock, srv) = common::setup().await;

    Mock::given(method("GET"))
        .and(path("/pullzone/123/safehop/statistics"))
        .and(header("AccessKey", common::TEST_KEY))
        .and(query_param("dateFrom", "2026-09-14T00:00:00Z"))
        .and(query_param("dateTo", "2026-10-14T23:59:59Z"))
        .and(query_param("hourly", "false"))
        .respond_with(ResponseTemplate::new(200).set_body_json(common::fixture("stats_safehop")))
        .mount(&mock)
        .await;

    let args = PullzoneFeatureStatisticsArgs {
        pullzone_id: 123,
        kind: FeatureKind::Safehop,
        date_from: Some("2026-09-14".to_string()),
        date_to: Some("2026-10-14".to_string()),
        hourly: Some(false),
        include_series: Some(true),
    };

    let res = srv
        .bunny_pullzone_feature_statistics(Parameters(args))
        .await
        .unwrap();
    common::assert_ok(&res);
    let val = common::json(&res);

    assert_eq!(val.get("pullzone_id"), Some(&json!(123)));
    assert_eq!(val.get("kind"), Some(&json!("safehop")));

    let totals = val.get("totals").expect("totals object");
    assert_eq!(totals.get("total_requests_retried"), Some(&json!(25.0)));
    assert_eq!(totals.get("total_requests_saved"), Some(&json!(20.0)));
    assert_eq!(totals.get("requests_retried_sum"), Some(&json!(25)));
    assert_eq!(totals.get("requests_saved_sum"), Some(&json!(20)));

    let series = val.get("series").expect("series object");
    assert!(series.get("requests_retried").is_some());
    assert!(series.get("requests_saved").is_some());
}

// covers: TOOL-ST-02
#[tokio::test]
async fn pullzone_feature_statistics_validation_and_api_errors() {
    let (mock, srv) = common::setup().await;

    // pullzone_id == 0
    let res = srv
        .bunny_pullzone_feature_statistics(Parameters(PullzoneFeatureStatisticsArgs {
            pullzone_id: 0,
            kind: FeatureKind::Optimizer,
            date_from: None,
            date_to: None,
            hourly: None,
            include_series: None,
        }))
        .await
        .unwrap();
    common::assert_err(&res, "pullzone_id must be >= 1");

    // Inverted date range
    let res = srv
        .bunny_pullzone_feature_statistics(Parameters(PullzoneFeatureStatisticsArgs {
            pullzone_id: 123,
            kind: FeatureKind::Optimizer,
            date_from: Some("2026-10-15".into()),
            date_to: Some("2026-10-14".into()),
            hourly: None,
            include_series: None,
        }))
        .await
        .unwrap();
    common::assert_err(&res, "date_from must be <= date_to");

    // 400 Bad Request
    Mock::given(method("GET"))
        .and(path("/pullzone/123/optimizer/statistics"))
        .respond_with(ResponseTemplate::new(400))
        .mount(&mock)
        .await;

    let res = srv
        .bunny_pullzone_feature_statistics(Parameters(PullzoneFeatureStatisticsArgs {
            pullzone_id: 123,
            kind: FeatureKind::Optimizer,
            date_from: Some("2026-09-14".into()),
            date_to: Some("2026-10-14".into()),
            hourly: None,
            include_series: None,
        }))
        .await
        .unwrap();
    common::assert_err(&res, "400");
}
