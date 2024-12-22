use actix_web::{
    get, http, middleware::Logger, post, web, App, HttpServer, Responder, ResponseError, Result,
};
use bytes::Bytes;
use func_gg::runtime::handler;
use futures::StreamExt;

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

    let (tx, rx) = tokio::sync::mpsc::channel::<Bytes>(1);

    actix_web::rt::spawn(async move {
        while let Some(item) = body.next().await {
            match item {
                Ok(chunk) => {
                    if let Err(e) = tx.send(chunk).await {
                        eprintln!("Error while sending chunk: {:?}", e);
                        break;
                    }
                }
                Err(e) => {
                    eprintln!("Error while reading body: {:?}", e);
                    break;
                }
            }
        }
    });

    handler(binary, rx).await?;

    Ok("done")
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
