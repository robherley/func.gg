use actix_web::{
    http, middleware::Logger, post, rt::spawn, web, App, HttpResponse, HttpServer, Responder,
    ResponseError, Result,
};
use func_gg::{
    runtime::Sandbox,
    streams::{InputStream, OutputStream},
};
use futures::StreamExt;
use log::error;

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
    let binary = include_bytes!("/Users/robherley/dev/webfunc-handler/dist/main.wasm");
    let mut sandbox = Sandbox::new(binary.to_vec())?;

    let (stdin, input_tx) = InputStream::new();

    // collect input from request body
    spawn(async move {
        while let Some(item) = body.next().await {
            input_tx.send(item?).await?;
        }
        Ok::<(), Error>(())
    });

    let (stdout, output_rx, mut first_write_rx) = OutputStream::new();

    // invoke the function
    spawn(async move {
        sandbox.call(stdin, stdout).await?;
        Ok::<(), Error>(())
    });

    let content_type = match first_write_rx.recv().await {
        Some(b'{') => "application/json",
        Some(b'<') => "text/html",
        _ => "text/plain",
    };

    Ok(HttpResponse::Ok().content_type(content_type).streaming(
        tokio_stream::wrappers::ReceiverStream::new(output_rx)
            .map(|item| Ok::<_, Error>(actix_web::web::Bytes::from(item))),
    ))
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init_from_env(env_logger::Env::default().default_filter_or("info"));

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
