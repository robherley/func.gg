use actix_web::{
    get, http, middleware::Logger, post, web, App, HttpResponse, HttpServer, Responder,
    ResponseError, Result,
};
use func_gg::{
    runtime::handler,
    streams::{ReceiverStream, SenderStream},
};
use futures::StreamExt;
use log::{info, warn};
use tokio::sync::mpsc::channel;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("runtime: {0}")]
    Runtime(#[from] func_gg::runtime::Error),
}

impl ResponseError for Error {
    fn status_code(&self) -> http::StatusCode {
        http::StatusCode::INTERNAL_SERVER_ERROR
    }
}

#[post("/")] // note: default payload limit is 256kB from actix-web, but is configurable with PayloadConfig
async fn handle(mut body: web::Payload) -> Result<impl Responder, Error> {
    let binary = include_bytes!("/Users/robherley/dev/webfunc-handler/dist/main.wasm");

    let (stdin, req_tx) = ReceiverStream::new();

    actix_web::rt::spawn(async move {
        while let Some(item) = body.next().await {
            match item {
                Ok(chunk) => {
                    if let Err(e) = req_tx.send(chunk).await {
                        warn!("unable to send chunk: {:?}", e);
                        break;
                    }
                }
                Err(e) => {
                    warn!("payload error: {:?}", e);
                    break;
                }
            }
        }
    });

    let (stdout, mut res_rx) = SenderStream::new();

    let (body_tx, body_rx) = channel::<Result<actix_web::web::Bytes, actix_web::Error>>(1);
    let stream = tokio_stream::wrappers::ReceiverStream::new(body_rx);

    actix_web::rt::spawn(async move {
        while let Some(item) = res_rx.recv().await {
            let chunk = actix_web::web::Bytes::from(item);
            info!("sending chunk: {:?}", chunk);
            if let Err(e) = body_tx.send(Ok(chunk)).await {
                warn!("unable to send chunk: {:?}", e);
                break;
            }
        }
    });

    actix_web::rt::spawn(async move {
        if let Err(e) = handler(binary, stdin, stdout).await {
            warn!("handler error: {:?}", e);
        }
    });

    // TODO(robherley): join handles? and proper error handling

    Ok(HttpResponse::Ok().streaming(stream))
}

#[get("/")]
async fn hello() -> impl Responder {
    "Hello world!"
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init_from_env(env_logger::Env::default().default_filter_or("info"));

    let addr = format!(
        "{}:{}",
        std::env::var("HOST").unwrap_or("127.0.0.1".into()),
        std::env::var("PORT").unwrap_or("8080".into()),
    );

    HttpServer::new(|| {
        App::new()
            .wrap(Logger::new("%r %s %Dms"))
            .service(hello)
            .service(handle)
    })
    .bind(addr)?
    .run()
    .await
}
