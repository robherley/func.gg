use axum::Router;
use tracing::info;

pub async fn serve(addr: &str) {
    let router = Router::new().fallback(handler);
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, router).await.unwrap();
}

async fn handler() -> &'static str {
    info!("Handling request");
    "Hello from funcd!"
}