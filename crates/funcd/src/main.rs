mod config;
mod ipc;
mod runtime;
mod server;

use anyhow::Result;
use std::sync::Arc;
use tokio::sync::oneshot;
use tokio::time::timeout;
use tracing::{error, info};

#[tokio::main]
async fn main() -> Result<()> {
    config::install_crypto()?;
    let cfg = config::load()?;
    cfg.init_tracing();

    let (ready_tx, ready_rx) = oneshot::channel();
    let socket = ipc::Socket::bind(&cfg.socket_path, ready_tx)?;
    tokio::spawn(async move {
        if let Err(e) = socket.listen().await {
            error!("unix socket listener error: {}", e);
        }
    });

    let mut proc = runtime::Process::new(
        &cfg.handler_path,
        &cfg.script_path,
        &cfg.socket_path,
        cfg.bun_path.as_ref(),
    );
    tokio::spawn(async move {
        if let Err(e) = proc.spawn().await {
            error!("runtime spawn error: {}", e);
        }

        if let Err(e) = proc.wait().await {
            error!("runtime wait error: {}", e);
        }
    });

    let upstream_port = match timeout(cfg.ready_timeout(), ready_rx).await {
        Ok(Ok(port)) => port,
        Ok(Err(e)) => anyhow::bail!("failed to receive server port: {}", e),
        Err(_) => anyhow::bail!(
            "timeout waiting for server port after {} seconds",
            cfg.ready_timeout_seconds
        ),
    };

    let proxy = Arc::new(server::Proxy::new("localhost".to_string(), upstream_port));

    info!(upstream = proxy.upstream, "initializing proxy");

    lambda_http::run(lambda_http::service_fn(move |req| {
        let proxy = Arc::clone(&proxy);
        async move { proxy.handle(req).await }
    }))
    .await
    .map_err(|e| anyhow::anyhow!("lambda runtime error: {}", e))?;

    Ok(())
}
