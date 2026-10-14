//! Shared wiremock harness (design §10). Each test crate uses a subset of these helpers.
#![allow(dead_code, clippy::expect_used, clippy::unwrap_used)]

use std::time::Duration;

use bunnycdn_mcp::client::BunnyClient;
use bunnycdn_mcp::config::Config;
use bunnycdn_mcp::secret::Secret;
use bunnycdn_mcp::server::BunnyServer;
use rmcp::model::CallToolResult;
use url::Url;
use wiremock::MockServer;

pub const TEST_KEY: &str = "test-key";

/// Client pointed at `base` with key `test-key`. Retry delays are capped at zero so retry paths
/// run instantly.
pub fn client_for(base: &str) -> BunnyClient {
    let cfg = Config {
        api_key: Secret::new(TEST_KEY),
        base_url: Url::parse(base).expect("valid mock url"),
    };
    BunnyClient::new(&cfg)
        .expect("client builds")
        .with_max_retry_delay(Duration::ZERO)
}

/// Mock Bunny API plus a raw client pointed at it.
pub async fn setup_client() -> (MockServer, BunnyClient) {
    let mock = MockServer::start().await;
    let client = client_for(&mock.uri());
    (mock, client)
}

/// Mock Bunny API plus a `BunnyServer` pointed at it. Call tool methods directly:
/// `srv.bunny_purge_url(Parameters(args)).await.unwrap()`.
pub async fn setup() -> (MockServer, BunnyServer) {
    let (mock, client) = setup_client().await;
    (mock, BunnyServer::new(client))
}

/// Text of the first content block.
pub fn text(r: &CallToolResult) -> String {
    r.content
        .first()
        .and_then(|c| c.as_text())
        .map(|t| t.text.clone())
        .expect("first content block is text")
}

/// First content block parsed as JSON.
pub fn json(r: &CallToolResult) -> serde_json::Value {
    let t = text(r);
    serde_json::from_str(&t).unwrap_or_else(|e| panic!("tool output is not JSON ({e}): {t}"))
}

pub fn assert_ok(r: &CallToolResult) {
    assert_ne!(
        r.is_error,
        Some(true),
        "expected success, got error: {}",
        text(r)
    );
}

pub fn assert_err(r: &CallToolResult, contains: &str) {
    let t = text(r);
    assert_eq!(r.is_error, Some(true), "expected error, got success: {t}");
    assert!(
        t.contains(contains),
        "error {t:?} does not contain {contains:?}"
    );
}

/// `tests/fixtures/<name>.json`.
pub fn fixture(name: &str) -> serde_json::Value {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(format!("{name}.json"));
    let raw = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("cannot read fixture {}: {e}", path.display()));
    serde_json::from_str(&raw).unwrap_or_else(|e| panic!("invalid fixture {}: {e}", path.display()))
}
