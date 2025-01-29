use actix_web::HttpResponseBuilder;
use actix_web::{middleware::Logger, post, rt::spawn, web, App, HttpServer, Responder, Result};
use funcgg::http::Error;
use funcgg::runtime::HTTPHead;
use funcgg::{
    runtime::Sandbox,
    streams::{InputStream, OutputStream},
};
use futures::StreamExt;
use log::{error, warn};
use std::sync::{Arc, Mutex};
use tokio::sync::{mpsc, oneshot};

// tokio_util::sync::CancellationToken
// https://tokio.rs/tokio/topics/shutdown
#[post("/")] // note: default payload limit is 256kB from actix-web, but is configurable with PayloadConfig
async fn handle(mut body: web::Payload) -> Result<impl Responder, Error> {
    let binary =
        include_bytes!("../examples/rust/target/wasm32-wasip2/release/funcgg_example_rust.wasm");
    let mut sandbox = Sandbox::new(binary.to_vec())?;

    let (stdin, input_tx) = InputStream::new();

    spawn(async move {
        while let Some(item) = body.next().await {
            if let Ok(item) = item {
                if let Err(e) = input_tx.send(item).await {
                    error!("unable to send chunk: {:?}", e);
                    break;
                }
            }
        }
        Ok::<(), Error>(())
    });

    let (stdout, mut output_rx) = OutputStream::new();
    let (body_tx, body_rx) = mpsc::channel::<Result<actix_web::web::Bytes, actix_web::Error>>(1);
    let (first_write_tx, first_write_rx) = oneshot::channel::<()>();

    spawn({
        let mut first_write_tx = Some(first_write_tx);
        async move {
            while let Some(item) = output_rx.recv().await {
                if let Some(first_write_tx) = first_write_tx.take() {
                    let _ = first_write_tx.send(());
                }
                if let Err(err) = body_tx.send(Ok(item)).await {
                    warn!("unable to send chunk: {:?}", err);
                    break;
                }
            }
        }
    });

    let head = Arc::new(Mutex::new(HTTPHead::default()));

    spawn({
        let head = head.clone();
        async move {
            if let Err(err) = sandbox.call(stdin, stdout, head).await {
                error!("sandbox error: {:?}", err);
            }
            Ok::<(), Error>(())
        }
    });

    _ = first_write_rx.await;

    let head = match head.lock() {
        Ok(h) => h.clone(),
        Err(err) => {
            error!("unable to unwrap head: {:?}", err);
            HTTPHead::default()
        }
    };

    let mut builder: HttpResponseBuilder = head.into();
    Ok(builder.streaming(tokio_stream::wrappers::ReceiverStream::new(body_rx)))
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .filter_module("wasmtime_wasi", log::LevelFilter::Warn)
        .filter_module("tracing", log::LevelFilter::Warn)
        .init();

    let addr = format!(
        "{}:{}",
        std::env::var("HOST").unwrap_or("127.0.0.1".into()),
        std::env::var("PORT").unwrap_or("8080".into()),
    );

    HttpServer::new(|| App::new().wrap(Logger::new("%r %s %Dms")).service(handle))
        .bind(addr)?
        .run()
        .await
}
