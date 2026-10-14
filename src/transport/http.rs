//! Streamable HTTP transport (design §11, REQ-TR-02..06).

use std::future::Future;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use axum::Router;
use axum::routing::get;
use rmcp::transport::streamable_http_server::session::local::LocalSessionManager;
use rmcp::transport::streamable_http_server::{StreamableHttpServerConfig, StreamableHttpService};
use tokio_util::sync::CancellationToken;

use crate::server::BunnyServer;

/// Options for the Streamable HTTP transport.
#[derive(Debug, Clone)]
pub struct HttpOptions {
    pub listen: SocketAddr,
    pub allowed_hosts: Vec<String>,
}

/// Builds the axum router for the Streamable HTTP transport.
///
/// Routes:
/// - `/mcp` -> Streamable HTTP service (with session manager and host validation)
/// - `/healthz` -> returns "ok" (200 OK)
pub fn router(
    server: BunnyServer,
    opts: &HttpOptions,
    cancellation_token: CancellationToken,
) -> Router {
    let mut config = StreamableHttpServerConfig::default();
    config.cancellation_token = cancellation_token;
    config.allowed_hosts.extend(opts.allowed_hosts.clone());

    let service: StreamableHttpService<BunnyServer, LocalSessionManager> =
        StreamableHttpService::new(
            move || Ok(server.clone()),
            Arc::new(LocalSessionManager::default()),
            config,
        );

    Router::new()
        .route("/healthz", get(|| async { "ok" }))
        .nest_service("/mcp", service)
}

/// Serves the transport using an existing TCP listener and a custom shutdown signal.
///
/// Exposed for testing and custom lifecycles.
pub async fn serve_with_shutdown(
    listener: tokio::net::TcpListener,
    server: BunnyServer,
    opts: HttpOptions,
    shutdown: impl Future<Output = ()> + Send + 'static,
) -> Result<(), std::io::Error> {
    let cancellation_token = CancellationToken::new();
    let app = router(server, &opts, cancellation_token.clone());

    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel();
    let ct_for_signal = cancellation_token.clone();

    let signal_task = tokio::spawn(async move {
        shutdown.await;
        tracing::info!("shutting down HTTP transport");
        ct_for_signal.cancel();
        let _ = shutdown_tx.send(());
    });

    let serve_fut = axum::serve(listener, app)
        .with_graceful_shutdown(async move {
            let _ = shutdown_rx.await;
        })
        .into_future();

    tokio::pin!(serve_fut);
    let res = tokio::select! {
        res = &mut serve_fut => {
            signal_task.abort();
            res
        }
        _ = cancellation_token.cancelled() => {
            match tokio::time::timeout(Duration::from_secs(5), serve_fut).await {
                Ok(res) => res,
                Err(_) => {
                    tracing::warn!("graceful shutdown timed out after 5s; forcing exit");
                    Ok(())
                }
            }
        }
    };

    res
}

/// Serves the MCP Streamable HTTP transport on the configured address.
///
/// Listens on `opts.listen`, handling requests until SIGINT or SIGTERM is received.
/// On shutdown signal, connection acceptance stops and active connections are given
/// up to 5 seconds to drain cleanly (REQ-TR-06).
pub async fn serve(server: BunnyServer, opts: HttpOptions) -> Result<(), std::io::Error> {
    let listener = tokio::net::TcpListener::bind(opts.listen).await?;
    serve_with_shutdown(listener, server, opts, shutdown_signal()).await
}

async fn shutdown_signal() {
    let ctrl_c = async {
        let _ = tokio::signal::ctrl_c().await;
    };

    #[cfg(unix)]
    let terminate = async {
        match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
            Ok(mut sig) => {
                sig.recv().await;
            }
            Err(e) => {
                tracing::warn!("failed to install SIGTERM handler: {e}");
                std::future::pending::<()>().await;
            }
        }
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}
