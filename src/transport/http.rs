//! Streamable HTTP transport (design §11). Stub created by T02; implemented by T70.

use std::net::SocketAddr;

use crate::server::BunnyServer;

pub struct HttpOptions {
    pub listen: SocketAddr,
    pub allowed_hosts: Vec<String>,
}

pub async fn serve(server: BunnyServer, opts: HttpOptions) -> Result<(), std::io::Error> {
    let _ = (server, opts);
    Err(std::io::Error::other("http transport not implemented yet"))
}
