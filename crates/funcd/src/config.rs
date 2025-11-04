use anyhow::Result;
use figment::{
    Figment,
    providers::{Env, Format, Toml},
};
use serde::{Deserialize, Serialize};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::path::PathBuf;
use std::time;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

const ENV_PREFIX: &str = "FUNCD_";
const CONFIG_FILE: &str = "funcd.toml";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    /// Log directive for the application, analogous to RUST_LOG
    pub log: String,

    /// Path to the handler TypeScript file
    pub handler_path: PathBuf,

    /// Path to the user script file
    pub script_path: PathBuf,

    /// Unix socket path for IPC
    pub socket_path: PathBuf,

    /// HTTP server host
    pub http_host: IpAddr,

    /// HTTP server port
    pub http_port: u16,

    /// Timeout in seconds for waiting for the runtime process to be ready
    pub ready_timeout_seconds: u64,

    /// Explicit path to the bun binary
    pub bun_path: Option<PathBuf>,
}

impl Config {
    pub fn http_addr(&self) -> SocketAddr {
        SocketAddr::new(self.http_host, self.http_port)
    }

    pub fn ready_timeout(&self) -> time::Duration {
        time::Duration::from_secs(self.ready_timeout_seconds)
    }

    pub fn init_tracing(&self) {
        let env_filter = tracing_subscriber::EnvFilter::builder().parse_lossy(&self.log);
        tracing_subscriber::registry()
            .with(env_filter)
            .with(
                tracing_subscriber::fmt::layer()
                    .with_target(false)
                    .with_line_number(true)
                    .with_file(true),
            )
            .init();
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            log: "info".to_string(),
            handler_path: PathBuf::from("/tmp/handler.ts"),
            script_path: PathBuf::from("/tmp/script.ts"),
            socket_path: PathBuf::from("/tmp/funcd.sock"),
            http_host: IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
            http_port: 8080,
            ready_timeout_seconds: 5,
            bun_path: None,
        }
    }
}

pub fn load() -> Result<Config> {
    let cfg = Figment::new()
        .merge(Toml::file(CONFIG_FILE))
        .merge(Env::prefixed(ENV_PREFIX))
        .extract()?;
    Ok(cfg)
}
