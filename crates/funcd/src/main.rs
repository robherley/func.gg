mod server;

use std::env;
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::from_default_env())
        .with(
            tracing_subscriber::fmt::layer()
                .with_target(false)
                .with_line_number(true)
                .with_file(true),
        )
        .init();
    
    let addr = format!(
        "{}:{}",
        env::var("HOST").unwrap_or("127.0.0.1".into()),
        env::var("PORT").unwrap_or("8081".into()),
    );
    
    info!("funcd on {}", &addr);
    server::serve(&addr).await;
}
