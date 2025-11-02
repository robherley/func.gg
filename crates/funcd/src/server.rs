use anyhow::Result;
use axum::Router;
use axum_reverse_proxy::ReverseProxy;
use tracing::info;

pub async fn proxy(addr: &str, proxy_port: u16) -> Result<()> {
    let process_host = format!("http://localhost:{}", proxy_port);
    let proxy = ReverseProxy::new("/", &process_host);
    let app: Router = proxy.into();
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    info!(component = "http", addr = %addr, proxy_port, "listening");
    axum::serve(listener, app).await?;
    Ok(())
}
