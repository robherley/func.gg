use anyhow::Result;
use quinn::{Connection, Endpoint};
use rustls::pki_types::{CertificateDer, PrivateKeyDer};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::RwLock;
use tracing::{error, info, warn};

pub struct Server {
    endpoint: Endpoint,
}

impl Server {
    pub async fn new(bind_addr: SocketAddr) -> Result<Self> {
        let server_config = Self::configure_server()?;
        let endpoint = Endpoint::server(server_config, bind_addr)?;

        Ok(Server { endpoint })
    }

    fn configure_server() -> Result<quinn::ServerConfig> {
        let cert = rcgen::generate_simple_self_signed(vec!["localhost".into()])?;
        let cert_der = cert.cert.der();
        let priv_key = cert.key_pair.serialize_der();

        let cert_chain = vec![CertificateDer::from(cert_der.to_vec())];
        let priv_key = PrivateKeyDer::Pkcs8(priv_key.into());

        let server_crypto = rustls::ServerConfig::builder()
            .with_no_client_auth()
            .with_single_cert(cert_chain, priv_key)?;

        let server_crypto = quinn::crypto::rustls::QuicServerConfig::try_from(server_crypto)?;
        let mut config = quinn::ServerConfig::with_crypto(Arc::new(server_crypto));
        let mut transport = quinn::TransportConfig::default();
        transport.max_concurrent_bidi_streams(100u32.into());
        transport.keep_alive_interval(Some(std::time::Duration::from_secs(5)));
        transport.max_idle_timeout(Some(std::time::Duration::from_secs(60).try_into()?));
        config.transport_config(Arc::new(transport));

        Ok(config)
    }

    pub async fn run(&self, target_addr: SocketAddr) -> Result<()> {
        info!("listening for QUIC on {}", self.endpoint.local_addr()?);
        info!("listening for TCP on {}", target_addr);

        let tcp_listener = TcpListener::bind(target_addr).await?;
        info!("accepting TCP connections on {}", target_addr);

        let active_connection: Arc<RwLock<Option<Connection>>> = Arc::new(RwLock::new(None));

        let quic_connection = active_connection.clone();
        let endpoint = self.endpoint.clone();
        tokio::spawn(async move {
            loop {
                match endpoint.accept().await {
                    Some(incoming) => match incoming.await {
                        Ok(connection) => {
                            info!("client connected from {}", connection.remote_address());
                            let mut conn_lock = quic_connection.write().await;
                            *conn_lock = Some(connection.clone());
                            drop(conn_lock);

                            let quic_connection_clone = quic_connection.clone();
                            tokio::spawn(async move {
                                connection.closed().await;
                                warn!("QUIC connection closed");
                                let mut conn_lock = quic_connection_clone.write().await;
                                *conn_lock = None;
                            });
                        }
                        Err(e) => {
                            error!("failed to accept QUIC connection: {}", e);
                        }
                    },
                    None => {
                        info!("QUIC endpoint closed");
                        break;
                    }
                }
            }
        });

        loop {
            let (tcp_stream, remote_addr) = tcp_listener.accept().await?;
            info!("accepting TCP connection from {}", remote_addr);

            let conn_lock = active_connection.read().await;
            match conn_lock.as_ref() {
                Some(connection) => {
                    let conn = connection.clone();
                    drop(conn_lock);
                    tokio::spawn(async move {
                        if let Err(e) = handle_tcp_connection(tcp_stream, conn).await {
                            error!("TCP connection error: {}", e);
                        }
                    });
                }
                None => {
                    warn!("no active QUIC connection, dropping TCP {}", remote_addr);
                    drop(conn_lock);
                }
            }
        }
    }
}

async fn handle_tcp_connection(tcp_stream: TcpStream, connection: Connection) -> Result<()> {
    info!("opening QUIC stream for TCP connection");

    let (mut send, mut recv) = connection.open_bi().await?;
    let (mut tcp_read, mut tcp_write) = tcp_stream.into_split();

    let tcp_2_quic = tokio::io::copy(&mut tcp_read, &mut send);
    let quic_2_tcp = tokio::io::copy(&mut recv, &mut tcp_write);

    tokio::select! {
        res = tcp_2_quic => {
            if let Err(e) = res {
                error!("TCP to QUIC copy error: {}", e);
            }
        }
        res = quic_2_tcp => {
            if let Err(e) = res {
                error!("QUIC to TCP copy error: {}", e);
            }
        }
    }

    Ok(())
}
