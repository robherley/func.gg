use std::fs;
use anyhow::Result;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::net::UnixListener;
use tracing::info;

pub const SOCKET_PATH: &str = "/tmp/funcd.sock";

pub fn create_socket() -> Result<UnixListener> {
    if fs::metadata(SOCKET_PATH).is_ok() {
        info!("removing existing socket at {}", SOCKET_PATH);
        fs::remove_file(SOCKET_PATH)?;
    }
    
    let socket = UnixListener::bind(SOCKET_PATH)?;
    info!("unix socket created at {}", SOCKET_PATH);
    
    Ok(socket)
}

pub async fn listen(socket: UnixListener) -> Result<()> {
    info!("socket listening on {}", SOCKET_PATH);
    
    loop {
        match socket.accept().await {
            Ok((stream, _addr)) => {
                info!("new connection on unix socket");
                
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