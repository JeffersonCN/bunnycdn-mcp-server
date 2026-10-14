//! BunnyClient behavior against a mock Bunny API (REQ-CFG-04, REQ-CLI-*).

mod common;

use bunnycdn_mcp::client::BunnyError;
use reqwest::Method;
use serde_json::{Value, json};
use wiremock::matchers::{body_json, header, method, path, query_param};
use wiremock::{Mock, ResponseTemplate};

use common::{TEST_KEY, client_for, setup_client};

// covers: REQ-CFG-04
#[tokio::test]
async fn sends_access_key_accept_and_user_agent() {
    let (mock, client) = setup_client().await;
    Mock::given(method("GET"))
        .and(path("/pullzone/1"))
        .and(query_param("includeCertificate", "false"))
        .and(header("AccessKey", TEST_KEY))
        .and(header("Accept", "application/json"))
        .and(header(
            "User-Agent",
            concat!("bunnycdn-mcp/", env!("CARGO_PKG_VERSION")),
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"Id": 1})))
        .expect(1)
        .mount(&mock)
        .await;

    let v: Value = client
        .get("/pullzone/1", &[("includeCertificate", "false".into())])
        .await
        .unwrap();
    assert_eq!(v, json!({"Id": 1}));
}

// covers: REQ-CLI-04
#[tokio::test]
async fn status_401_is_unauthorized() {
    let (mock, client) = setup_client().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(401))
        .expect(1)
        .mount(&mock)
        .await;

    let err = client.get::<Value>("/pullzone", &[]).await.unwrap_err();
    assert!(matches!(err, BunnyError::Unauthorized), "{err:?}");
    assert!(err.to_tool_message().contains("BUNNY_API_KEY"));
}

// covers: REQ-CLI-04
#[tokio::test]
async fn status_404_is_not_found_with_path() {
    let (mock, client) = setup_client().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(404))
        .expect(1)
        .mount(&mock)
        .await;

    let err = client.get::<Value>("/pullzone/99", &[]).await.unwrap_err();
    assert!(
        matches!(&err, BunnyError::NotFound { path } if path == "/pullzone/99"),
        "{err:?}"
    );
    assert_eq!(
        err.to_tool_message(),
        "Not found (404): /pullzone/99. Verify the id with bunny_pullzone_list."
    );
}

// covers: REQ-CLI-04
#[tokio::test]
async fn status_400_preserves_api_error_data() {
    let (mock, client) = setup_client().await;
    Mock::given(method("POST"))
        .and(path("/pullzone/1"))
        .respond_with(ResponseTemplate::new(400).set_body_json(json!({
            "ErrorKey": "pullzone.validation",
            "Field": "OriginUrl",
            "Message": "The origin URL is invalid"
        })))
        .expect(1)
        .mount(&mock)
        .await;

    let err = client
        .post::<Value>("/pullzone/1", &[], Some(&json!({"OriginUrl": "nope"})))
        .await
        .unwrap_err();
    match &err {
        BunnyError::Api {
            status,
            key,
            field,
            message,
        } => {
            assert_eq!(*status, 400);
            assert_eq!(key.as_deref(), Some("pullzone.validation"));
            assert_eq!(field.as_deref(), Some("OriginUrl"));
            assert_eq!(message.as_deref(), Some("The origin URL is invalid"));
        }
        other => panic!("unexpected {other:?}"),
    }
    assert_eq!(
        err.to_tool_message(),
        "Bunny API error 400 (pullzone.validation) field=OriginUrl: The origin URL is invalid"
    );
}

// covers: REQ-CLI-02
#[tokio::test]
async fn status_429_retries_three_times_then_rate_limited() {
    let (mock, client) = setup_client().await;
    Mock::given(method("POST"))
        .and(path("/purge"))
        .respond_with(ResponseTemplate::new(429).insert_header("Retry-After", "1"))
        .expect(4) // first attempt + 3 retries, even for POST
        .mount(&mock)
        .await;

    let err = client
        .post::<Value>("/purge", &[("url", "https://x.b-cdn.net/a".into())], None)
        .await
        .unwrap_err();
    assert!(matches!(err, BunnyError::RateLimited), "{err:?}");
    assert!(err.to_tool_message().contains("after 3 retries"));
}

// covers: REQ-CLI-02
#[tokio::test]
async fn status_429_then_success() {
    let (mock, client) = setup_client().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(429).insert_header("Retry-After", "1"))
        .up_to_n_times(2)
        .expect(2)
        .mount(&mock)
        .await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([1, 2])))
        .expect(1)
        .mount(&mock)
        .await;

    let v: Value = client.get("/country", &[]).await.unwrap();
    assert_eq!(v, json!([1, 2]));
}

// covers: REQ-CLI-03
#[tokio::test]
async fn status_5xx_on_get_is_retried() {
    let (mock, client) = setup_client().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(503))
        .expect(4)
        .mount(&mock)
        .await;

    let err = client.get::<Value>("/pullzone", &[]).await.unwrap_err();
    assert!(
        matches!(err, BunnyError::Api { status: 503, .. }),
        "{err:?}"
    );
}

// covers: REQ-CLI-03
#[tokio::test]
async fn status_5xx_on_get_recovers() {
    let (mock, client) = setup_client().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(500))
        .up_to_n_times(1)
        .expect(1)
        .mount(&mock)
        .await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"ok": 1})))
        .expect(1)
        .mount(&mock)
        .await;

    let v: Value = client.get("/pullzone", &[]).await.unwrap();
    assert_eq!(v, json!({"ok": 1}));
}

// covers: REQ-CLI-03
#[tokio::test]
async fn status_5xx_on_post_is_not_retried() {
    let (mock, client) = setup_client().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(500).set_body_json(json!({"Message": "boom"})))
        .expect(1)
        .mount(&mock)
        .await;

    let err = client
        .post::<Value>("/pullzone/1/purgeCache", &[], None)
        .await
        .unwrap_err();
    assert_eq!(err.to_tool_message(), "Bunny API error 500: boom");
}

// covers: REQ-CLI-03
#[tokio::test]
async fn network_error_on_get_is_retried_then_reported() {
    // Port 1 on loopback refuses connections immediately.
    let client = client_for("http://127.0.0.1:1");
    let err = client.get::<Value>("/pullzone", &[]).await.unwrap_err();
    assert!(matches!(err, BunnyError::Network(_)), "{err:?}");
    assert!(
        err.to_tool_message()
            .starts_with("Network error talking to Bunny API:")
    );
    assert!(!err.to_tool_message().contains(TEST_KEY));
    // The cause from the source chain is included, not just "error sending request".
    let msg = err.to_tool_message();
    assert!(
        msg.len() > "Network error talking to Bunny API: error sending request".len(),
        "{msg}"
    );
    assert!(!msg.contains("127.0.0.1:1/pullzone"), "{msg}");
}

// covers: REQ-CLI-05
#[tokio::test]
async fn empty_body_is_null() {
    let (mock, client) = setup_client().await;
    Mock::given(method("POST"))
        .and(path("/pullzone/1/purgeCache"))
        .respond_with(ResponseTemplate::new(204))
        .expect(1)
        .mount(&mock)
        .await;
    Mock::given(method("POST"))
        .and(path("/pullzone/2/purgeCache"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&mock)
        .await;

    let v: Value = client
        .post("/pullzone/1/purgeCache", &[], None)
        .await
        .unwrap();
    assert_eq!(v, Value::Null);
    let v = client
        .request(Method::POST, "/pullzone/2/purgeCache", &[], None)
        .await
        .unwrap();
    assert_eq!(v, Value::Null);
}

#[tokio::test]
async fn invalid_json_is_decode_error() {
    let (mock, client) = setup_client().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200).set_body_string("not json"))
        .mount(&mock)
        .await;

    let err = client.get::<Value>("/pullzone", &[]).await.unwrap_err();
    assert!(matches!(err, BunnyError::Decode(_)), "{err:?}");
}

#[tokio::test]
async fn bodiless_post_and_delete_send_content_length_zero() {
    let (mock, client) = setup_client().await;
    Mock::given(method("POST"))
        .and(path("/pullzone/1/purgeCache"))
        .and(header("Content-Length", "0"))
        .respond_with(ResponseTemplate::new(204))
        .expect(1)
        .mount(&mock)
        .await;
    Mock::given(method("DELETE"))
        .and(path("/pullzone/1"))
        .and(header("Content-Length", "0"))
        .respond_with(ResponseTemplate::new(204))
        .expect(1)
        .mount(&mock)
        .await;

    let v: Value = client
        .post("/pullzone/1/purgeCache", &[], None)
        .await
        .unwrap();
    assert_eq!(v, Value::Null);
    let v: Value = client.delete("/pullzone/1", None).await.unwrap();
    assert_eq!(v, Value::Null);
}

#[tokio::test]
async fn delete_sends_json_body() {
    let (mock, client) = setup_client().await;
    let body = json!({"Hostname": "cdn.example.com"});
    Mock::given(method("DELETE"))
        .and(path("/pullzone/5/removeHostname"))
        .and(header("AccessKey", TEST_KEY))
        .and(header("Content-Type", "application/json"))
        .and(body_json(&body))
        .respond_with(ResponseTemplate::new(204))
        .expect(1)
        .mount(&mock)
        .await;

    let v: Value = client
        .delete("/pullzone/5/removeHostname", Some(&body))
        .await
        .unwrap();
    assert_eq!(v, Value::Null);
}
