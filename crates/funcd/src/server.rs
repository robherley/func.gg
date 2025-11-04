use std::net::SocketAddr;

use anyhow::Result;
use axum::Router;
use axum_reverse_proxy::ReverseProxy;
use tracing::info;

pub async fn proxy(addr: &SocketAddr, func_port: u16) -> Result<()> {
    let func_host = format!("http://localhost:{}", func_port);
    let proxy = ReverseProxy::new("/", &func_host);
    let app: Router = proxy.into();
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    info!(component = "http", addr = %addr, func_port, "listening");
    axum::serve(listener, app).await?;
    Ok(())
}
