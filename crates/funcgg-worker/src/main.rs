mod routes;
mod workers;

use std::sync::Arc;
use tower_http::trace::TraceLayer;
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::from_default_env())
        .with(
            tracing_subscriber::fmt::layer()
                .with_target(false)
                .with_thread_ids(true)
                .with_line_number(true)
                .with_file(true),
        )
        .init();

    let addr = format!(
        "{}:{}",
        std::env::var("HOST").unwrap_or("127.0.0.1".into()),
        std::env::var("PORT").unwrap_or("8081".into()),
    );

    let pool_size = std::env::var("WORKER_POOL_SIZE")
        .ok()
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(
            // default to 2x + 1 logical cores, assume we're mostly i/o bound
            std::thread::available_parallelism()
                .map(|n| 2 * n.get() + 1)
                .unwrap_or(1),
        );

    info!("Creating worker pool with {} workers", pool_size);
    let worker_pool = Arc::new(workers::Pool::new(pool_size));

    let app = routes::build(worker_pool).layer(
        TraceLayer::new_for_http()
            .make_span_with(|request: &axum::http::Request<_>| {
                tracing::info_span!(
                    "http_request",
                    method = %request.method(),
                    uri = %request.uri(),
                    version = ?request.version(),
                )
            })
            .on_response(
                |response: &axum::http::Response<_>,
                 latency: std::time::Duration,
                 _span: &tracing::Span| {
                    tracing::info!(
                        status = response.status().as_u16(),
                        latency = ?latency,
                        "response"
                    );
                },
            ),
    );

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    info!("Server running on {}", addr);

    axum::serve(listener, app).await?;

    Ok(())
}
