use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::net::UnixListener;
use tokio::sync::oneshot;
use tracing::{debug, info, warn};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", content = "payload")]
#[serde(rename_all = "snake_case")]
pub enum Message {
    Started,
    Ready,
    Error { error: String },
}

pub struct Socket {
    path: PathBuf,
    listener: UnixListener,
    port_tx: Arc<Mutex<Option<oneshot::Sender<()>>>>,
}

impl Socket {
    pub fn bind<P: AsRef<Path>>(path: P, ready_tx: oneshot::Sender<()>) -> Result<Self> {
        let path = path.as_ref().to_path_buf();

        if fs::metadata(&path).is_ok() {
            debug!(socket = %path.display(), "removing existing socket");
            fs::remove_file(&path)?;
        }

        let listener = UnixListener::bind(&path)?;
        debug!(socket = %path.display(), "socket created");

        Ok(Self {
            path,
            listener,
            port_tx: Arc::new(Mutex::new(Some(ready_tx))),
        })
    }

    pub async fn listen(self) -> Result<()> {
        info!(
            component = "socket",
            socket = %self.path.display(),
            "listening"
        );

        loop {
            match self.listener.accept().await {
                Ok((stream, _)) => {
                    info!("new connection on unix socket");

                    let port_tx = Arc::clone(&self.port_tx);
                    tokio::spawn(async move {
                        let reader = BufReader::new(stream);
                        let mut lines = reader.lines();

                        while let Ok(Some(line)) = lines.next_line().await {
                            match serde_json::from_str::<Message>(&line) {
                                Ok(msg) => Self::handle_message(Arc::clone(&port_tx), msg).await,
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

    pub async fn handle_message(
        port_tx: Arc<Mutex<Option<oneshot::Sender<()>>>>,
        message: Message,
    ) {
        info!(message = ?message, "received message");

        match message {
            Message::Started => {}
            Message::Ready => {
                if let Ok(mut guard) = port_tx.lock()
                    && let Some(tx) = guard.take()
                {
                    let _ = tx.send(());
                }
            }
            Message::Error { error } => {
                warn!(error = %error, "error occurred");
            }
        }
    }
}
