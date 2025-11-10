use anyhow::{Result, anyhow};
use figment::{
    Figment,
    providers::{Env, Format, Toml},
};
use serde::{Deserialize, Serialize};
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

    /// Runtime paths to sockets, binaries and scripts
    pub paths: Paths,

    /// Timeout in seconds for waiting for the runtime process to be ready
    pub ready_timeout_seconds: u64,

    /// Enable response streaming
    pub response_streaming: bool,
}

impl Config {
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
            ready_timeout_seconds: 5,
            response_streaming: false,
            paths: Paths::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Paths {
    pub bun: PathBuf,
    pub socket: PathBuf,
    pub entry_point: PathBuf,
    pub user_script: PathBuf,
}

impl Default for Paths {
    fn default() -> Self {
        Self {
            bun: PathBuf::from("/opt/bun"),
            socket: PathBuf::from("/tmp/funcd.sock"),
            entry_point: PathBuf::from("/var/task/entry_point.ts"),
            user_script: PathBuf::from("/var/task/user_script.ts"),
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

pub fn install_crypto() -> Result<()> {
    rustls::crypto::aws_lc_rs::default_provider()
        .install_default()
        .map_err(|e| anyhow!("unable to set crypto provider: {:?}", e))
}
