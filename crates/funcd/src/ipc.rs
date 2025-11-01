use anyhow::Result;
use std::fs;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::net::UnixListener;
use tracing::info;

pub struct Socket {
    addr: String,
    listener: UnixListener,
}

impl Socket {
    pub fn bind(addr: String) -> Result<Self> {
        if fs::metadata(&addr).is_ok() {
            info!(socket = %addr, "socket removed");
            fs::remove_file(&addr)?;
        }

        let listener = UnixListener::bind(&addr)?;
        info!(socket = %addr, "socket created");

        Ok(Self { addr, listener })
    }

    pub async fn listen(&self) -> Result<()> {
        info!(
            component = "socket",
            socket = %self.addr,
            "listening"
        );

        loop {
            match self.listener.accept().await {
                Ok((stream, addr)) => {
                    info!("new connection on unix socket: {:?}", addr);

                    tokio::spawn(async move {
                        let reader = BufReader::new(stream);
                        let mut lines = reader.lines();

                        while let Ok(Some(line)) = lines.next_line().await {
                            info!("received: {}", line);
                        }

                        info!("connection closed");
                    });
                }
                Err(e) => {
                    info!("error accepting connection: {}", e);
                }
            }
        }
    }
}
