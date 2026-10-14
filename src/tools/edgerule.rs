//! `edgerule_router` stub created by T02. The domain task replaces this whole file's router with a
//! `#[tool_router(router = edgerule_router, vis = "pub(crate)")]` block (see `tools/util.rs`).

use rmcp::handler::server::router::tool::ToolRouter;

use crate::server::BunnyServer;

impl BunnyServer {
    pub(crate) fn edgerule_router() -> ToolRouter<BunnyServer> {
        ToolRouter::new()
    }
}
