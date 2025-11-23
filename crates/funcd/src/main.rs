mod config;
mod ipc;
mod responder;
mod runtime;

use std::net::SocketAddr;
use tokio::sync::oneshot;
use tokio::time::timeout;
use tracing::{error, info};

async fn boot(cfg: &config::Config) -> anyhow::Result<()> {
    let (ready_tx, ready_rx) = oneshot::channel();
    let socket = ipc::Socket::bind(&cfg.paths.msg_socket, ready_tx)?;
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

    let port = match timeout(cfg.ready_timeout(), ready_rx).await {
        Ok(Ok(port)) => port,
        Ok(Err(e)) => anyhow::bail!("failed to start runtime: {}", e),
        Err(_) => anyhow::bail!(
            "timeout waiting for server to be ready after {} seconds",
            cfg.ready_timeout_seconds
        ),
    };
    info!(dur = ?start.elapsed(), runtime_port = port, "runtime ready");

    let remote: SocketAddr = "127.0.0.1:4433".parse()?;

    let tun = funnel::Client::new(remote).await?;
    let tun_addr = format!("127.0.0.1:{}", port).parse()?;

    if cfg.is_local() {
        return tun.run(tun_addr).await;
    }

    tokio::spawn(async move {
        if let Err(err) = tun.run(tun_addr).await {
            error!("tunnel failed: {}", err)
        }
    });

    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    config::install_crypto()?;
    let cfg = config::load()?;
    cfg.init_tracing();

    let result = boot(&cfg).await;
    responder::respond(cfg.mode, result).await
}
