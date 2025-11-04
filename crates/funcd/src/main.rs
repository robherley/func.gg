mod config;
mod ipc;
mod runtime;
mod server;

use anyhow::Result;
use tokio::sync::oneshot;
use tokio::time::timeout;
use tracing::{error, info};

#[tokio::main]
async fn main() -> Result<()> {
    let cfg = config::load()?;
    cfg.init_tracing();

    info!(
        http_addr = %cfg.http_addr(),
        socket_path = %cfg.socket_path.display(),
        handler_path = %cfg.handler_path.display(),
        script_path = %cfg.script_path.display(),
        "initializing funcd"
    );

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

    let func_port = match timeout(cfg.ready_timeout(), ready_rx).await {
        Ok(Ok(port)) => port,
        Ok(Err(e)) => anyhow::bail!("failed to receive server port: {}", e),
        Err(_) => anyhow::bail!("timeout waiting for server port"),
    };

    info!("will proxy requests to port: {}", func_port);
    server::proxy(&cfg.http_addr(), func_port).await?;
    Ok(())
}
