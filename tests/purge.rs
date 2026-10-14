//! Integration tests for Purge tools (specs/tools/purge.md, REQ-PG-01, REQ-PG-02).

mod common;

use bunnycdn_mcp::tools::purge::{PurgePullzoneArgs, PurgeUrlArgs};
use common::{TEST_KEY, assert_err, assert_ok, json, setup};
use rmcp::handler::server::wrapper::Parameters;
use serde_json::json;
use wiremock::matchers::{body_json, header, method, path, query_param};
use wiremock::{Match, Mock, Request, ResponseTemplate};

struct QueryStringMatcher<F>(F)
where
    F: Fn(&str) -> bool + Send + Sync;

impl<F> Match for QueryStringMatcher<F>
where
    F: Fn(&str) -> bool + Send + Sync,
{
    fn matches(&self, request: &Request) -> bool {
        let query = request.url.query().unwrap_or("");
        (self.0)(query)
    }
}

// covers: TOOL-PG-01
#[tokio::test]
async fn purge_url_success_default() {
    let (mock, srv) = setup().await;

    Mock::given(method("POST"))
        .and(path("/purge"))
        .and(query_param("url", "https://cdn.example.com/img/*"))
        .and(header("AccessKey", TEST_KEY))
        .and(header("Accept", "application/json"))
        .and(QueryStringMatcher(|q| {
            // Document exact query: verify whether * is percent-encoded or verbatim
            // and verify that async and exactPath are omitted when false.
            !q.contains("async") && !q.contains("exactPath")
        }))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&mock)
        .await;

    let res = srv
        .bunny_purge_url(Parameters(PurgeUrlArgs {
            url: "https://cdn.example.com/img/*".to_string(),
            r#async: None,
            exact_path: None,
        }))
        .await
        .unwrap();

    assert_ok(&res);
    let v = json(&res);
    assert_eq!(
        v,
        json!({
            "ok": true,
            "url": "https://cdn.example.com/img/*",
            "async": false,
        })
    );
}

// covers: TOOL-PG-01
#[tokio::test]
async fn purge_url_query_encoding_inspection() {
    let (mock, srv) = setup().await;

    Mock::given(method("POST"))
        .and(path("/purge"))
        .and(header("AccessKey", TEST_KEY))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&mock)
        .await;

    let res = srv
        .bunny_purge_url(Parameters(PurgeUrlArgs {
            url: "https://cdn.example.com/img/*".to_string(),
            r#async: Some(false),
            exact_path: Some(false),
        }))
        .await
        .unwrap();

    assert_ok(&res);

    let requests = mock.received_requests().await.unwrap();
    assert_eq!(requests.len(), 1);
    let req = &requests[0];
    let raw_query = req.url.query().expect("query must be present");

    // Print raw query so it appears in test log
    eprintln!("Wiremock received raw query: {raw_query}");
    // Verify query param "url" can be looked up by wiremock
    assert_eq!(
        req.url
            .query_pairs()
            .find(|(k, _)| k == "url")
            .map(|(_, v)| v),
        Some("https://cdn.example.com/img/*".into())
    );
}

// covers: TOOL-PG-01
#[tokio::test]
async fn purge_url_with_async_and_exact_path() {
    let (mock, srv) = setup().await;

    Mock::given(method("POST"))
        .and(path("/purge"))
        .and(query_param("url", "https://cdn.example.com/img/"))
        .and(query_param("async", "true"))
        .and(query_param("exactPath", "true"))
        .and(header("AccessKey", TEST_KEY))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&mock)
        .await;

    let res = srv
        .bunny_purge_url(Parameters(PurgeUrlArgs {
            url: "https://cdn.example.com/img/".to_string(),
            r#async: Some(true),
            exact_path: Some(true),
        }))
        .await
        .unwrap();

    assert_ok(&res);
    let v = json(&res);
    assert_eq!(
        v,
        json!({
            "ok": true,
            "url": "https://cdn.example.com/img/",
            "async": true,
        })
    );
}

// covers: TOOL-PG-01
#[tokio::test]
async fn purge_url_invalid_without_scheme_or_host() {
    let (mock, srv) = setup().await;

    // Missing scheme
    let res = srv
        .bunny_purge_url(Parameters(PurgeUrlArgs {
            url: "cdn.example.com/img/*".to_string(),
            r#async: None,
            exact_path: None,
        }))
        .await
        .unwrap();
    assert_err(&res, "Invalid argument");

    // Invalid scheme (not http or https)
    let res = srv
        .bunny_purge_url(Parameters(PurgeUrlArgs {
            url: "ftp://cdn.example.com/img/*".to_string(),
            r#async: None,
            exact_path: None,
        }))
        .await
        .unwrap();
    assert_err(&res, "Invalid argument");

    // Missing host
    let res = srv
        .bunny_purge_url(Parameters(PurgeUrlArgs {
            url: "http://".to_string(),
            r#async: None,
            exact_path: None,
        }))
        .await
        .unwrap();
    assert_err(&res, "Invalid argument");

    // Wiremock should not receive any requests
    assert_eq!(mock.received_requests().await.unwrap().len(), 0);
}

// covers: TOOL-PG-01
#[tokio::test]
async fn purge_url_unauthorized_401() {
    let (mock, srv) = setup().await;

    Mock::given(method("POST"))
        .and(path("/purge"))
        .respond_with(ResponseTemplate::new(401))
        .expect(1)
        .mount(&mock)
        .await;

    let res = srv
        .bunny_purge_url(Parameters(PurgeUrlArgs {
            url: "https://cdn.example.com/img/*".to_string(),
            r#async: None,
            exact_path: None,
        }))
        .await
        .unwrap();

    assert_err(&res, "401");
}

// covers: TOOL-PG-02
#[tokio::test]
async fn purge_pullzone_without_tag_success() {
    let (mock, srv) = setup().await;

    Mock::given(method("POST"))
        .and(path("/pullzone/123/purgeCache"))
        .and(header("AccessKey", TEST_KEY))
        .and(header("Content-Length", "0"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&mock)
        .await;

    let res = srv
        .bunny_purge_pullzone(Parameters(PurgePullzoneArgs {
            id: 123,
            cache_tag: None,
        }))
        .await
        .unwrap();

    assert_ok(&res);
    let v = json(&res);
    assert_eq!(
        v,
        json!({
            "ok": true,
            "id": 123,
            "scope": "all",
        })
    );
}

// covers: TOOL-PG-02
#[tokio::test]
async fn purge_pullzone_with_tag_success() {
    let (mock, srv) = setup().await;

    Mock::given(method("POST"))
        .and(path("/pullzone/123/purgeCache"))
        .and(header("AccessKey", TEST_KEY))
        .and(body_json(json!({ "CacheTag": "v2" })))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&mock)
        .await;

    let res = srv
        .bunny_purge_pullzone(Parameters(PurgePullzoneArgs {
            id: 123,
            cache_tag: Some("v2".to_string()),
        }))
        .await
        .unwrap();

    assert_ok(&res);
    let v = json(&res);
    assert_eq!(
        v,
        json!({
            "ok": true,
            "id": 123,
            "scope": "tag",
            "cache_tag": "v2",
        })
    );
}

// covers: TOOL-PG-02
#[tokio::test]
async fn purge_pullzone_invalid_id() {
    let (mock, srv) = setup().await;

    let res = srv
        .bunny_purge_pullzone(Parameters(PurgePullzoneArgs {
            id: 0,
            cache_tag: None,
        }))
        .await
        .unwrap();

    assert_err(&res, "Invalid argument");
    assert_eq!(mock.received_requests().await.unwrap().len(), 0);
}

// covers: TOOL-PG-02
#[tokio::test]
async fn purge_pullzone_not_found_404() {
    let (mock, srv) = setup().await;

    Mock::given(method("POST"))
        .and(path("/pullzone/999/purgeCache"))
        .respond_with(ResponseTemplate::new(404))
        .expect(1)
        .mount(&mock)
        .await;

    let res = srv
        .bunny_purge_pullzone(Parameters(PurgePullzoneArgs {
            id: 999,
            cache_tag: None,
        }))
        .await
        .unwrap();

    assert_err(&res, "404");
}
