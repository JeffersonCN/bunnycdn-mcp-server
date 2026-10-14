//! Tests for reference tools: `bunny_region_list` and `bunny_country_list` (design §10, C6).

mod common;

use bunnycdn_mcp::tools::reference::{CountryListArgs, RegionListArgs};
use rmcp::handler::server::wrapper::Parameters;
use serde_json::json;
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, ResponseTemplate};

// covers: TOOL-RF-01
#[tokio::test]
async fn region_list_fixture_mapping() {
    let (mock, srv) = common::setup().await;

    Mock::given(method("GET"))
        .and(path("/region"))
        .and(header("AccessKey", common::TEST_KEY))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(common::fixture("reference_regions")),
        )
        .mount(&mock)
        .await;

    let res = srv
        .bunny_region_list(Parameters(RegionListArgs { continent: None }))
        .await
        .unwrap();

    common::assert_ok(&res);
    let data = common::json(&res);
    assert_eq!(
        data,
        json!({
            "regions": [
                {
                    "id": 1,
                    "name": "Europe: Frankfurt, DE",
                    "code": "DE",
                    "continent": "EU",
                    "country": "DE",
                    "price_per_gb": 0.01
                },
                {
                    "id": 2,
                    "name": "North America: New York, US",
                    "code": "NY",
                    "continent": "NA",
                    "country": "US",
                    "price_per_gb": 0.01
                },
                {
                    "id": 3,
                    "name": "Asia: Singapore, SG",
                    "code": "SG",
                    "continent": "AS",
                    "country": "SG",
                    "price_per_gb": 0.03
                }
            ]
        })
    );
}

// covers: TOOL-RF-01
#[tokio::test]
async fn region_list_filters() {
    let (mock, srv) = common::setup().await;

    Mock::given(method("GET"))
        .and(path("/region"))
        .and(header("AccessKey", common::TEST_KEY))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(common::fixture("reference_regions")),
        )
        .mount(&mock)
        .await;

    // Filter by continent case-insensitively ("eu")
    let res_eu = srv
        .bunny_region_list(Parameters(RegionListArgs {
            continent: Some("eu".to_string()),
        }))
        .await
        .unwrap();

    common::assert_ok(&res_eu);
    let data_eu = common::json(&res_eu);
    assert_eq!(
        data_eu,
        json!({
            "regions": [
                {
                    "id": 1,
                    "name": "Europe: Frankfurt, DE",
                    "code": "DE",
                    "continent": "EU",
                    "country": "DE",
                    "price_per_gb": 0.01
                }
            ]
        })
    );

    // Filter with no match
    let res_none = srv
        .bunny_region_list(Parameters(RegionListArgs {
            continent: Some("XX".to_string()),
        }))
        .await
        .unwrap();

    common::assert_ok(&res_none);
    let data_none = common::json(&res_none);
    assert_eq!(data_none, json!({ "regions": [] }));
}

// covers: TOOL-RF-01
#[tokio::test]
async fn region_list_500_maps_to_error() {
    let (mock, srv) = common::setup().await;

    Mock::given(method("GET"))
        .and(path("/region"))
        .and(header("AccessKey", common::TEST_KEY))
        .respond_with(ResponseTemplate::new(500).set_body_string("Internal Server Error"))
        .mount(&mock)
        .await;

    let res = srv
        .bunny_region_list(Parameters(RegionListArgs { continent: None }))
        .await
        .unwrap();

    common::assert_err(&res, "Bunny API error 500");
}

// covers: TOOL-RF-02
#[tokio::test]
async fn country_list_fixture_mapping() {
    let (mock, srv) = common::setup().await;

    Mock::given(method("GET"))
        .and(path("/country"))
        .and(header("AccessKey", common::TEST_KEY))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(common::fixture("reference_countries")),
        )
        .mount(&mock)
        .await;

    let res = srv
        .bunny_country_list(Parameters(CountryListArgs { search: None }))
        .await
        .unwrap();

    common::assert_ok(&res);
    let data = common::json(&res);
    assert_eq!(
        data,
        json!({
            "countries": [
                {
                    "iso": "BR",
                    "name": "Brazil"
                },
                {
                    "iso": "DE",
                    "name": "Germany",
                    "eu": true
                },
                {
                    "iso": "US",
                    "name": "United States"
                }
            ]
        })
    );
}

// covers: TOOL-RF-02
#[tokio::test]
async fn country_list_filters() {
    let (mock, srv) = common::setup().await;

    Mock::given(method("GET"))
        .and(path("/country"))
        .and(header("AccessKey", common::TEST_KEY))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(common::fixture("reference_countries")),
        )
        .mount(&mock)
        .await;

    // Filter by name case-insensitively ("brazil")
    let res_name = srv
        .bunny_country_list(Parameters(CountryListArgs {
            search: Some("brazil".to_string()),
        }))
        .await
        .unwrap();

    common::assert_ok(&res_name);
    let data_name = common::json(&res_name);
    assert_eq!(
        data_name,
        json!({
            "countries": [
                {
                    "iso": "BR",
                    "name": "Brazil"
                }
            ]
        })
    );

    // Filter by ISO code case-insensitively ("de")
    let res_iso = srv
        .bunny_country_list(Parameters(CountryListArgs {
            search: Some("de".to_string()),
        }))
        .await
        .unwrap();

    common::assert_ok(&res_iso);
    let data_iso = common::json(&res_iso);
    assert_eq!(
        data_iso,
        json!({
            "countries": [
                {
                    "iso": "DE",
                    "name": "Germany",
                    "eu": true
                }
            ]
        })
    );

    // Filter by partial substring ("uni")
    let res_uni = srv
        .bunny_country_list(Parameters(CountryListArgs {
            search: Some("uni".to_string()),
        }))
        .await
        .unwrap();

    common::assert_ok(&res_uni);
    let data_uni = common::json(&res_uni);
    assert_eq!(
        data_uni,
        json!({
            "countries": [
                {
                    "iso": "US",
                    "name": "United States"
                }
            ]
        })
    );

    // Filter with no match
    let res_none = srv
        .bunny_country_list(Parameters(CountryListArgs {
            search: Some("nonexistent".to_string()),
        }))
        .await
        .unwrap();

    common::assert_ok(&res_none);
    let data_none = common::json(&res_none);
    assert_eq!(data_none, json!({ "countries": [] }));
}

// covers: TOOL-RF-02
#[tokio::test]
async fn country_list_500_maps_to_error() {
    let (mock, srv) = common::setup().await;

    Mock::given(method("GET"))
        .and(path("/country"))
        .and(header("AccessKey", common::TEST_KEY))
        .respond_with(ResponseTemplate::new(500).set_body_string("Internal Server Error"))
        .mount(&mock)
        .await;

    let res = srv
        .bunny_country_list(Parameters(CountryListArgs { search: None }))
        .await
        .unwrap();

    common::assert_err(&res, "Bunny API error 500");
}
