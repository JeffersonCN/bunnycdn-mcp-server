//! MCP tools, one module per domain (specs/tools/README.md). Each module defines a
//! `#[tool_router(router = <domain>_router, vis = "pub(crate)")]` block on `BunnyServer`.

pub mod access;
pub mod edgerule;
pub mod hostname;
pub mod pullzone;
pub mod purge;
pub mod reference;
pub mod stats;
pub mod util;
