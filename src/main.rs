use axum::{http::StatusCode, response::IntoResponse, routing::get, Router};
use tower_http::trace::{self, TraceLayer};
use tracing::{info, Level};
use wunc::Runtime;

macro_rules! fail {
    ($c:expr, $e:expr) => {
        return ($c, $e.to_string())
    };
}

async fn handle() -> impl IntoResponse {
    let binary = include_bytes!("../handler.component.wasm");

    let mut runtime = match Runtime::new(binary) {
        Ok(v) => v,
        Err(e) => fail!(StatusCode::INTERNAL_SERVER_ERROR, e),
    };

    let (response, store) = match runtime.handle().await {
        Ok(v) => v,
        Err(e) => fail!(StatusCode::INTERNAL_SERVER_ERROR, e),
    };

    let data = match std::str::from_utf8(&store.data().data) {
        Ok(s) => s.to_string(),
        Err(e) => fail!(StatusCode::INTERNAL_SERVER_ERROR, e),
    };

    let code = match StatusCode::try_from(response.status) {
        Ok(v) => v,
        Err(e) => fail!(StatusCode::INTERNAL_SERVER_ERROR, e),
    };

    (code, data)
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt().compact().init();

    let addr = format!(
        "{}:{}",
        std::env::var("HOST").unwrap_or("127.0.0.1".into()),
        std::env::var("PORT").unwrap_or("8080".into()),
    );

    let router = Router::new().route("/", get(handle)).layer(
        TraceLayer::new_for_http()
            .make_span_with(trace::DefaultMakeSpan::new().level(Level::INFO))
            .on_response(trace::DefaultOnResponse::new().level(Level::INFO)),
    );

    let listener = tokio::net::TcpListener::bind(addr).await?;
    info!("listening on {}", listener.local_addr()?);
    axum::serve(listener, router).await?;

    Ok(())
}
