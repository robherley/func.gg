use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::net::UnixListener;
use tracing::info;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", content = "payload")]
#[serde(rename_all = "snake_case")]
pub enum Message {
    Ping,
    Ready { port: u16 },
}

pub struct Socket {
    path: PathBuf,
    listener: UnixListener,
}

impl Socket {
    pub fn bind<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref().to_path_buf();

        if fs::metadata(&path).is_ok() {
            info!(socket = %path.display(), "removing existing socket");
            fs::remove_file(&path)?;
        }

        let listener = UnixListener::bind(&path)?;
        info!(socket = %path.display(), "socket created");

        Ok(Self { path, listener })
    }

    pub async fn listen(&self) -> Result<()> {
        info!(
            component = "socket",
            socket = %self.path.display(),
            "listening"
        );

        loop {
            match self.listener.accept().await {
                Ok((stream, _)) => {
                    info!("new connection on unix socket");

                    tokio::spawn(async move {
                        let reader = BufReader::new(stream);
                        let mut lines = reader.lines();

                        while let Ok(Some(line)) = lines.next_line().await {
                            match serde_json::from_str::<Message>(&line) {
                                Ok(msg) => info!(message = ?msg, "received message"),
                                Err(e) => info!(error = %e, "failed to parse message"),
                            }
                        }

                        info!("connection closed");
                    });
                }
                Err(e) => {
                    info!(error = %e, "error accepting connection");
                }
            }
        }
    }
}
