//! HTTP transport integration tests (design §11, REQ-TR-02..06).

mod common;

use std::time::Duration;

use bunnycdn_mcp::server::{INSTRUCTIONS, SERVER_NAME};
use bunnycdn_mcp::transport::http::{HttpOptions, serve, serve_with_shutdown};

fn parse_sse_data(body: &str) -> serde_json::Value {
    for line in body.lines() {
        if let Some(rest) = line.strip_prefix("data: ")
            && let Ok(v) = serde_json::from_str::<serde_json::Value>(rest)
        {
            return v;
        }
    }
    panic!("No valid JSON data line found in SSE body:\n{body}");
}

// covers: REQ-TR-02
#[tokio::test]
async fn healthz_endpoint_returns_ok() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let server = bunnycdn_mcp::server::BunnyServer::new(common::client_for("http://127.0.0.1:1"));
    let opts = HttpOptions {
        listen: addr,
        allowed_hosts: vec![],
    };

    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel();
    let server_handle = tokio::spawn(async move {
        serve_with_shutdown(listener, server, opts, async move {
            let _ = shutdown_rx.await;
        })
        .await
        .unwrap();
    });

    let client = reqwest::Client::new();
    let response = client
        .get(format!("http://{addr}/healthz"))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 200);
    assert_eq!(response.text().await.unwrap(), "ok");

    let _ = shutdown_tx.send(());
    tokio::time::timeout(Duration::from_secs(2), server_handle)
        .await
        .unwrap()
        .unwrap();
}

// covers: REQ-TR-02, REQ-TR-05
#[tokio::test]
async fn mcp_initialize_and_tools_list_via_http() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let server = bunnycdn_mcp::server::BunnyServer::new(common::client_for("http://127.0.0.1:1"));
    let opts = HttpOptions {
        listen: addr,
        allowed_hosts: vec![],
    };

    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel();
    let server_handle = tokio::spawn(async move {
        serve_with_shutdown(listener, server, opts, async move {
            let _ = shutdown_rx.await;
        })
        .await
        .unwrap();
    });

    let client = reqwest::Client::new();
    let mcp_url = format!("http://{addr}/mcp");

    // 1. Send initialize request
    let init_payload = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "2025-03-26",
            "capabilities": {},
            "clientInfo": {
                "name": "test-client",
                "version": "1.0.0"
            }
        }
    });

    let init_res = client
        .post(&mcp_url)
        .header("Accept", "application/json, text/event-stream")
        .header("Content-Type", "application/json")
        .json(&init_payload)
        .send()
        .await
        .unwrap();

    assert_eq!(init_res.status(), 200);
    let session_id_hdr = init_res
        .headers()
        .get("mcp-session-id")
        .expect("mcp-session-id header present")
        .to_str()
        .unwrap()
        .to_string();

    let body = init_res.text().await.unwrap();
    let json = parse_sse_data(&body);
    assert_eq!(json["jsonrpc"], "2.0");
    assert_eq!(json["id"], 1);
    assert_eq!(json["result"]["serverInfo"]["name"], SERVER_NAME);
    assert_eq!(
        json["result"]["serverInfo"]["version"],
        env!("CARGO_PKG_VERSION")
    );
    assert_eq!(
        json["result"]["instructions"].as_str().unwrap(),
        INSTRUCTIONS
    );
    assert!(json["result"]["capabilities"]["tools"].is_object());

    // 2. Send initialized notification
    let initialized_payload = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "notifications/initialized"
    });

    let notif_res = client
        .post(&mcp_url)
        .header("Accept", "application/json, text/event-stream")
        .header("Content-Type", "application/json")
        .header("mcp-session-id", &session_id_hdr)
        .json(&initialized_payload)
        .send()
        .await
        .unwrap();

    assert_eq!(notif_res.status(), 202);

    // 3. Send tools/list request
    let tools_list_payload = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "tools/list"
    });

    let tools_res = client
        .post(&mcp_url)
        .header("Accept", "application/json, text/event-stream")
        .header("Content-Type", "application/json")
        .header("mcp-session-id", &session_id_hdr)
        .json(&tools_list_payload)
        .send()
        .await
        .unwrap();

    assert_eq!(tools_res.status(), 200);
    let tools_body = tools_res.text().await.unwrap();
    let tools_json = parse_sse_data(&tools_body);
    assert_eq!(tools_json["jsonrpc"], "2.0");
    assert_eq!(tools_json["id"], 2);
    let tools = tools_json["result"]["tools"].as_array().unwrap();
    assert!(tools.len() <= 24);

    let _ = shutdown_tx.send(());
    tokio::time::timeout(Duration::from_secs(2), server_handle)
        .await
        .unwrap()
        .unwrap();
}

// covers: REQ-TR-05
#[tokio::test]
async fn host_validation_rejects_evil_host_and_allows_allowed_hosts() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let server = bunnycdn_mcp::server::BunnyServer::new(common::client_for("http://127.0.0.1:1"));
    let opts = HttpOptions {
        listen: addr,
        allowed_hosts: vec!["custom.internal".to_string()],
    };

    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel();
    let server_handle = tokio::spawn(async move {
        serve_with_shutdown(listener, server, opts, async move {
            let _ = shutdown_rx.await;
        })
        .await
        .unwrap();
    });

    let client = reqwest::Client::new();
    let mcp_url = format!("http://{addr}/mcp");

    let init_payload = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "2025-03-26",
            "capabilities": {},
            "clientInfo": {
                "name": "test-client",
                "version": "1.0.0"
            }
        }
    });

    // Disallowed host should be rejected with 403 Forbidden
    let evil_res = client
        .post(&mcp_url)
        .header("Accept", "application/json, text/event-stream")
        .header("Content-Type", "application/json")
        .header("Host", "evil.example")
        .json(&init_payload)
        .send()
        .await
        .unwrap();

    assert_eq!(evil_res.status(), 403);

    // Custom allowed host should be accepted (status 200)
    let allowed_res = client
        .post(&mcp_url)
        .header("Accept", "application/json, text/event-stream")
        .header("Content-Type", "application/json")
        .header("Host", "custom.internal")
        .json(&init_payload)
        .send()
        .await
        .unwrap();

    assert_eq!(allowed_res.status(), 200);

    let _ = shutdown_tx.send(());
    tokio::time::timeout(Duration::from_secs(2), server_handle)
        .await
        .unwrap()
        .unwrap();
}

// covers: REQ-TR-06
#[tokio::test]
async fn graceful_shutdown_exits_within_five_seconds() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let server = bunnycdn_mcp::server::BunnyServer::new(common::client_for("http://127.0.0.1:1"));
    let opts = HttpOptions {
        listen: addr,
        allowed_hosts: vec![],
    };

    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel();
    let server_handle = tokio::spawn(async move {
        serve_with_shutdown(listener, server, opts, async move {
            let _ = shutdown_rx.await;
        })
        .await
        .unwrap();
    });

    // Make sure server is up
    let client = reqwest::Client::new();
    let health = client
        .get(format!("http://{addr}/healthz"))
        .send()
        .await
        .unwrap();
    assert_eq!(health.status(), 200);

    let start = std::time::Instant::now();
    let _ = shutdown_tx.send(());
    tokio::time::timeout(Duration::from_secs(5), server_handle)
        .await
        .expect("server did not shut down within 5 seconds")
        .expect("server task panicked");

    let elapsed = start.elapsed();
    assert!(
        elapsed <= Duration::from_secs(5),
        "shutdown took too long: {elapsed:?}"
    );
}

// covers: REQ-TR-03
#[tokio::test]
async fn serve_binds_and_runs() {
    // Test serve() entry point with an ephemeral loopback port
    let probe = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = probe.local_addr().unwrap();
    drop(probe);

    let server = bunnycdn_mcp::server::BunnyServer::new(common::client_for("http://127.0.0.1:1"));
    let opts = HttpOptions {
        listen: addr,
        allowed_hosts: vec![],
    };

    let handle = tokio::spawn(async move {
        let _ = serve(server, opts).await;
    });

    // Wait for the server to bind and respond to healthz
    let client = reqwest::Client::new();
    let mut started = false;
    for _ in 0..50 {
        if let Ok(res) = client.get(format!("http://{addr}/healthz")).send().await
            && res.status() == 200
        {
            started = true;
            break;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }

    assert!(started, "server failed to respond on {addr}");
    handle.abort();
}
