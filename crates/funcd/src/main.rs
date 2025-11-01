mod ipc;
mod runtime;
mod server;

use anyhow::Result;
use std::env;
use std::time::Duration;
use tokio::sync::oneshot;
use tokio::time::timeout;
use tracing::{error, info};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

const HANDLER_PATH: &str = "/Users/robherley/dev/func.gg/js/handler.ts";
const SCRIPT_PATH: &str = "/Users/robherley/dev/func.gg/examples/websocket.js";

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

    let (port_tx, port_rx) = oneshot::channel();
    let socket = ipc::Socket::bind(&socket_path, port_tx)?;
    tokio::spawn(async move {
        if let Err(e) = socket.listen().await {
            error!("unix socket listener error: {}", e);
        }
    });

    let mut proc = runtime::Process::new(HANDLER_PATH, SCRIPT_PATH, &socket_path);
    tokio::spawn(async move {
        if let Err(e) = proc.spawn().await {
            error!("runtime spawn error: {}", e);
        }

        if let Err(e) = proc.wait().await {
            error!("runtime wait error: {}", e);
        }
    });

    let http_port = match timeout(Duration::from_secs(5), port_rx).await {
        Ok(Ok(port)) => port,
        Ok(Err(e)) => anyhow::bail!("failed to receive server port: {}", e),
        Err(_) => anyhow::bail!("timeout waiting for server port"),
    };

    info!("will proxy requests to port: {}", http_port);

    server::serve(&http_addr, http_port).await?;
    Ok(())
}
