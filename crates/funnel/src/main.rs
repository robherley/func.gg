use anyhow::Result;
use clap::{Parser, Subcommand};
use funnel::{Client, Server};
use std::net::SocketAddr;

#[derive(Parser)]
#[command(name = "funnel")]
#[command(about = "A simple QUIC-based TCP tunnel")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Server {
        #[arg(short, long, default_value = "0.0.0.0:4433")]
        bind: SocketAddr,

        #[arg(short, long)]
        target: SocketAddr,
    },
    Client {
        #[arg(short, long, default_value = "127.0.0.1:8080")]
        bind: SocketAddr,

        #[arg(short, long)]
        server: SocketAddr,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    rustls::crypto::aws_lc_rs::default_provider()
        .install_default()
        .unwrap();

    let cli = Cli::parse();

    match cli.command {
        Commands::Server { bind, target } => {
            let server = Server::new(bind).await?;
            server.run(target).await?;
        }
        Commands::Client { bind, server } => {
            let client = Client::new(server).await?;
            client.run(bind).await?;
        }
    }

    Ok(())
}
