use actix_web::{
    http, middleware::Logger, post, rt::spawn, web, App, HttpResponse, HttpServer, Responder,
    ResponseError, Result,
};
use func_gg::{
    runtime::Sandbox,
    streams::{InputStream, OutputStream},
};
use futures::StreamExt;
use log::{error, warn};
use tokio::sync::mpsc::channel;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("runtime: {0}")]
    Runtime(#[from] func_gg::runtime::Error),
    #[error("payload: {0}")]
    Payload(#[from] actix_web::error::PayloadError),
    #[error("send: {0}")]
    Send(String),
}

impl<T> From<tokio::sync::mpsc::error::SendError<T>> for Error {
    fn from(err: tokio::sync::mpsc::error::SendError<T>) -> Self {
        Self::Send(err.to_string())
    }
}

impl ResponseError for Error {
    fn status_code(&self) -> http::StatusCode {
        http::StatusCode::INTERNAL_SERVER_ERROR
    }
}

// tokio_util::sync::CancellationToken
// https://tokio.rs/tokio/topics/shutdown
#[post("/")] // note: default payload limit is 256kB from actix-web, but is configurable with PayloadConfig
async fn handle(mut body: web::Payload) -> Result<impl Responder, Error> {
    let binary = include_bytes!("../examples/go-hello-world/dist/main.wasm");
    let mut sandbox = Sandbox::new(binary.to_vec())?;

    let (stdin, input_tx) = InputStream::new();

    spawn(async move {
        while let Some(item) = body.next().await {
            if let Err(e) = input_tx.send(item?).await {
                error!("unable to send chunk: {:?}", e);
                break;
            }
        }
        Ok::<(), Error>(())
    });

    let (stdout, mut output_rx) = OutputStream::new();
    let (body_tx, body_rx) = channel::<Result<actix_web::web::Bytes, actix_web::Error>>(1);
    let stream = tokio_stream::wrappers::ReceiverStream::new(body_rx);

    let (first_char_tx, first_char_rx) = tokio::sync::oneshot::channel::<u8>();

    spawn(async move {
        let mut first_char_tx = Some(first_char_tx);
        while let Some(item) = output_rx.recv().await {
            if let Some(tx) = first_char_tx.take() {
                if let Err(err) = tx.send(item[0]) {
                    warn!("failed to send first char: {:?}", err);
                }
            }

            if let Err(e) = body_tx.send(Ok(item)).await {
                warn!("unable to send chunk: {:?}", e);
                break;
            }
        }
    });

    spawn(async move {
        sandbox.call(stdin, stdout).await?;
        Ok::<(), Error>(())
    });

    let content_type = match first_char_rx.await {
        Ok(b'{') => "application/json",
        Ok(b'<') => "text/html",
        _ => "text/plain",
    };

    Ok(HttpResponse::Ok()
        .content_type(content_type)
        .streaming(stream))
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
