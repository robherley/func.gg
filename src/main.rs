use actix_web::{
    get, http, middleware::Logger, post, web, App, HttpResponse, HttpServer, Responder,
    ResponseError, Result,
};
use futures::StreamExt;
use webfunc::{runtime::Sandbox, stream};

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("webfunc error: {0}")]
    Runtime(#[from] webfunc::runtime::Error),
}

impl ResponseError for Error {
    fn status_code(&self) -> http::StatusCode {
        http::StatusCode::INTERNAL_SERVER_ERROR
    }
}

#[post("/")]
async fn handle(body: web::Payload) -> Result<impl Responder, Error> {
    let wasm = include_bytes!("/Users/robherley/dev/webfunc-handler/dist/main.wasm");

    // let mut runtime = Sandbox::new(wasm)?;
    // runtime.handle(body.into()).await?;

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
