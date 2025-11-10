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
    let socket = ipc::Socket::bind(&cfg.paths.socket, ready_tx)?;
    tokio::spawn(async move {
        if let Err(e) = socket.listen().await {
            error!("unix socket listener error: {}", e);
        }
    });

    let start = tokio::time::Instant::now();
    let mut proc = runtime::Process::new(cfg.paths.clone());
    tokio::spawn(async move {
        if let Err(e) = proc.spawn().await {
            error!("runtime spawn error: {}", e);
        }
        info!(dur = ?start.elapsed(), "runtime spawned");

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
    info!(dur = ?start.elapsed(), "runtime ready");

    let proxy = Arc::new(server::Proxy::new("localhost".to_string(), upstream_port));
    info!(
        upstream = proxy.upstream,
        streaming = cfg.response_streaming,
        "initializing proxy"
    );

    let res = if cfg.response_streaming {
        let svc_fn = lambda_http::service_fn(move |req| {
            let proxy = Arc::clone(&proxy);
            async move { proxy.handle_with_streaming_response(req).await }
        });
        lambda_http::run_with_streaming_response(svc_fn).await
    } else {
        let svc_fn = lambda_http::service_fn(move |req| {
            let proxy = Arc::clone(&proxy);
            async move { proxy.handle(req).await }
        });
        lambda_http::run(svc_fn).await
    };

    res.map_err(|e| anyhow::anyhow!("lambda runtime error: {}", e))?;
    Ok(())
}
