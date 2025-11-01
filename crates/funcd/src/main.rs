mod server;
mod ipc;

use std::env;
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use anyhow::Result;

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
    
    let addr = format!(
        "{}:{}",
        env::var("HOST").unwrap_or("127.0.0.1".into()),
        env::var("PORT").unwrap_or("8081".into()),
    );
    
    info!("initializing funcd");

    let socket = ipc::create_socket()?;
    tokio::spawn(async move {
        if let Err(e) = ipc::listen(socket).await {
            tracing::error!("unix socket listener error: {}", e);
        }
    });
    
    server::serve(&addr).await?;
    Ok(())
}
