//! MCP handshake over an in-memory duplex pipe, the same framing as stdio (REQ-TR-01).

mod common;

use bunnycdn_mcp::server::{BunnyServer, INSTRUCTIONS, SERVER_NAME};
use rmcp::ServiceExt;

// covers: REQ-TR-01
#[tokio::test]
async fn initialize_and_list_tools() {
    let server = BunnyServer::new(common::client_for("http://127.0.0.1:1"));
    let (server_io, client_io) = tokio::io::duplex(64 * 1024);
    let server_handle = tokio::spawn(async move {
        let running = server.serve(server_io).await.unwrap();
        running.waiting().await.unwrap();
    });

    let client = ().serve(client_io).await.unwrap();
    let info = client.peer_info().expect("server info after initialize");
    let implementation = info.server_info.as_ref().expect("serverInfo");
    assert_eq!(implementation.name, SERVER_NAME);
    assert_eq!(implementation.version, env!("CARGO_PKG_VERSION"));
    assert_eq!(info.instructions.as_deref(), Some(INSTRUCTIONS));
    assert!(info.capabilities.tools.is_some());

    // All domain routers are in place for Wave 2 / T90.
    let tools = client.list_all_tools().await.unwrap();
    assert_eq!(tools.len(), 24);

    client.cancel().await.unwrap();
    server_handle.await.unwrap();
}
