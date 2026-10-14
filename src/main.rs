//! CLI entry point (design §12): config → client → server → transport.

use std::net::SocketAddr;
use std::path::PathBuf;
use std::process::ExitCode;

use bunnycdn_mcp::client::{BunnyClient, install_crypto_provider};
use bunnycdn_mcp::config::Config;
use bunnycdn_mcp::server::BunnyServer;
use bunnycdn_mcp::transport::http::{self, HttpOptions};
use clap::{Parser, ValueEnum};
use rmcp::ServiceExt;
use tracing_subscriber::EnvFilter;

/// Exit code for configuration errors (missing/unreadable API key, bad base URL).
const EXIT_CONFIG: u8 = 2;

#[derive(Parser)]
#[command(
    name = "bunnycdn-mcp",
    version,
    about = "MCP server for the bunny.net CDN"
)]
struct Cli {
    /// File containing the Bunny account API key (overrides BUNNY_API_KEY).
    #[arg(long, value_name = "PATH")]
    api_key_file: Option<PathBuf>,

    /// MCP transport.
    #[arg(long, value_enum, default_value_t = Transport::Stdio)]
    transport: Transport,

    /// Listen address for the HTTP transport.
    #[arg(long, value_name = "ADDR", default_value = "127.0.0.1:8080")]
    listen: SocketAddr,

    /// Extra Host header value accepted by the HTTP transport (repeatable).
    #[arg(long = "allowed-host", value_name = "HOST")]
    allowed_hosts: Vec<String>,
}

#[derive(Clone, Copy, ValueEnum)]
enum Transport {
    Stdio,
    Http,
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> ExitCode {
    let cli = Cli::parse();

    // stdout carries the stdio protocol: logs go to stderr only (C3, REQ-CFG-06).
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("warn")),
        )
        .init();

    install_crypto_provider();

    // Validate the key before opening any transport (REQ-CFG-02).
    let config = match Config::load(cli.api_key_file.as_deref()) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("bunnycdn-mcp: {e}");
            return ExitCode::from(EXIT_CONFIG);
        }
    };
    let client = match BunnyClient::new(&config) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("bunnycdn-mcp: {}", e.to_tool_message());
            return ExitCode::from(EXIT_CONFIG);
        }
    };
    let server = BunnyServer::new(client);

    let result = match cli.transport {
        Transport::Stdio => serve_stdio(server).await,
        Transport::Http => {
            if !cli.listen.ip().is_loopback() {
                tracing::warn!(
                    "HTTP transport has NO authentication; anyone who can reach {} controls your Bunny account.",
                    cli.listen
                );
            }
            http::serve(
                server,
                HttpOptions {
                    listen: cli.listen,
                    allowed_hosts: cli.allowed_hosts,
                },
            )
            .await
            .map_err(|e| e.to_string())
        }
    };

    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("bunnycdn-mcp: {e}");
            ExitCode::FAILURE
        }
    }
}

async fn serve_stdio(server: BunnyServer) -> Result<(), String> {
    let service = server
        .serve(rmcp::transport::stdio())
        .await
        .map_err(|e| format!("MCP initialization failed: {e}"))?;
    service.waiting().await.map_err(|e| e.to_string())?;
    Ok(())
}
