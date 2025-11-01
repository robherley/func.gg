mod ipc;
mod runtime;
mod server;

use anyhow::Result;
use std::env;
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::from_default_env())
        .with(
            tracing_subscriber::fmt::layer()
                .with_target(false)
                .with_line_number(true)
                .with_file(true),
        )
        .init();

    let http_addr = format!(
        "{}:{}",
        env::var("FUNCD_HOST").unwrap_or("127.0.0.1".into()),
        env::var("FUNCD_PORT").unwrap_or("8081".into()),
    );

    let socket_path = env::var("FUNCD_SOCKET").unwrap_or("/tmp/funcd.sock".into());

    info!(http_addr = %http_addr, socket_path = %socket_path, "initializing funcd");

    let socket = ipc::Socket::bind(socket_path)?;
    tokio::spawn(async move {
        if let Err(e) = socket.listen().await {
            tracing::error!("unix socket listener error: {}", e);
        }
    });

    server::serve(&http_addr).await?;
    Ok(())
}
