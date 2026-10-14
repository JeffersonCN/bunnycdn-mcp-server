//! `BunnyServer`: MCP handler composing the per-domain tool routers (design §6).

use rmcp::handler::server::router::tool::ToolRouter;
use rmcp::model::{Implementation, ServerCapabilities, ServerConfig};
use rmcp::{ServerHandler, tool_handler};

use crate::client::BunnyClient;

pub const SERVER_NAME: &str = "bunnycdn-mcp";

/// Design §7.
pub const INSTRUCTIONS: &str =
    "Operate the bunny.net CDN (pull zones, cache purge, hostnames/SSL, edge rules, statistics).
IDs are numeric pull zone ids; find them with bunny_pullzone_list. Responses are summaries;
pass verbose=true on get tools for full objects. Write tools change production immediately.";

#[derive(Clone)]
pub struct BunnyServer {
    // Read by the domain tool modules (Wave 1); unused while every router is a stub.
    #[allow(dead_code)]
    pub(crate) client: BunnyClient,
    tool_router: ToolRouter<BunnyServer>,
}

impl BunnyServer {
    pub fn new(client: BunnyClient) -> Self {
        Self {
            client,
            tool_router: Self::pullzone_router()
                + Self::access_router()
                + Self::purge_router()
                + Self::hostname_router()
                + Self::edgerule_router()
                + Self::stats_router()
                + Self::reference_router(),
        }
    }
}

#[tool_handler(router = self.tool_router)]
impl ServerHandler for BunnyServer {
    fn get_info(&self) -> ServerConfig {
        ServerConfig::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(Implementation::new(SERVER_NAME, env!("CARGO_PKG_VERSION")))
            .with_instructions(INSTRUCTIONS)
    }
}
