use anyhow::Result;
use quinn::{Connection, Endpoint, RecvStream, SendStream};
use rustls::DigitallySignedStruct;
use rustls::client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier};
use rustls::pki_types::{CertificateDer, ServerName, UnixTime};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpStream;
use tracing::{error, info};

#[derive(Debug)]
struct SkipServerVerification;

impl ServerCertVerifier for SkipServerVerification {
    fn verify_server_cert(
        &self,
        _end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _server_name: &ServerName<'_>,
        _ocsp_response: &[u8],
        _now: UnixTime,
    ) -> Result<ServerCertVerified, rustls::Error> {
        Ok(ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        Ok(HandshakeSignatureValid::assertion())
    }

    fn verify_tls13_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        Ok(HandshakeSignatureValid::assertion())
    }

    fn supported_verify_schemes(&self) -> Vec<rustls::SignatureScheme> {
        vec![
            rustls::SignatureScheme::RSA_PKCS1_SHA256,
            rustls::SignatureScheme::ECDSA_NISTP256_SHA256,
            rustls::SignatureScheme::ED25519,
        ]
    }
}

pub struct Client {
    connection: Connection,
}

impl Client {
    pub async fn new(server_addr: SocketAddr) -> Result<Self> {
        let client_config = Self::configure_client()?;
        let mut endpoint = Endpoint::client("0.0.0.0:0".parse()?)?;
        endpoint.set_default_client_config(client_config);

        info!("connecting to QUIC server at {}", server_addr);
        let connection = endpoint.connect(server_addr, "localhost")?.await?;
        info!("established QUIC connection to {}", server_addr);

        Ok(Client { connection })
    }

    fn configure_client() -> Result<quinn::ClientConfig> {
        // TODO unsafe: use rcgen'd cert from the server
        let client_crypto = rustls::ClientConfig::builder()
            .dangerous()
            .with_custom_certificate_verifier(Arc::new(SkipServerVerification))
            .with_no_client_auth();

        let client_crypto = quinn::crypto::rustls::QuicClientConfig::try_from(client_crypto)?;
        let mut config = quinn::ClientConfig::new(Arc::new(client_crypto));
        let mut transport = quinn::TransportConfig::default();
        transport.max_concurrent_bidi_streams(100u32.into());
        transport.keep_alive_interval(Some(std::time::Duration::from_secs(5)));
        transport.max_idle_timeout(Some(std::time::Duration::from_secs(60).try_into()?));
        config.transport_config(Arc::new(transport));

        Ok(config)
    }

    pub async fn run(&self, local_service_addr: SocketAddr) -> Result<()> {
        info!(addr = ?local_service_addr, "forwarding to local service");

        loop {
            match self.connection.accept_bi().await {
                Ok((send, recv)) => {
                    info!("received new QUIC stream from server");

                    tokio::spawn(async move {
                        if let Err(e) = proxy_quic_to_tcp(send, recv, local_service_addr).await {
                            error!("proxy error: {}", e);
                        }
                    });
                }
                Err(quinn::ConnectionError::ApplicationClosed { .. }) => {
                    info!("connection closed by server");
                    break;
                }
                Err(e) => {
                    error!("error accepting bidirectional stream: {}", e);
                    break;
                }
            }
        }

        Ok(())
    }
}

async fn proxy_quic_to_tcp(
    mut send: SendStream,
    mut recv: RecvStream,
    local_service_addr: SocketAddr,
) -> Result<()> {
    info!("connecting to local service at {}", local_service_addr);

    let tcp_stream = TcpStream::connect(local_service_addr).await?;
    let (mut tcp_read, mut tcp_write) = tcp_stream.into_split();

    let quic_2_tcp = tokio::io::copy(&mut recv, &mut tcp_write);
    let tcp_2_quic = tokio::io::copy(&mut tcp_read, &mut send);

    tokio::select! {
        res = quic_2_tcp => {
            if let Err(e) = res {
                error!("QUIC to TCP copy error: {}", e);
            }
        }
        res = tcp_2_quic => {
            if let Err(e) = res {
                error!("TCP to QUIC copy error: {}", e);
            }
        }
    }

    Ok(())
}
