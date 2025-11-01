use anyhow::Result;
use axum::Router;
use tracing::info;

pub async fn serve(addr: &str) -> Result<()> {
    let router = Router::new().fallback(handler);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    info!("http listening on {}", &addr);
    axum::serve(listener, router).await?;
    Ok(())
}

async fn handler() -> &'static str {
    info!("Handling request");
    "Hello from funcd!"
}