use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};

use anyhow::Context;
use tokio::net::TcpListener;

pub async fn free_local_port() -> anyhow::Result<u16> {
    let socket = SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 0));
    let listener = TcpListener::bind(&socket)
        .await
        .with_context(|| format!("unable to bind to {}", socket))?;
    let socket_addr = listener.local_addr()?;

    Ok(socket_addr.port())
}
