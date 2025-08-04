use std::collections::HashMap;
use std::sync::Arc;

use axum::{body::Body, http::StatusCode, response::{Html, IntoResponse, Response}, routing::get, Router, extract::State};
use crate::runtime::HttpRequest;
use crate::worker_pool::WorkerPool;

pub async fn health() -> Html<&'static str> {
    Html("OK")
}

pub async fn invoke(State(pool): State<Arc<WorkerPool>>) -> Response {
    // TODO: temporary, eventually this should be served dynamically
    // possibly serve https://github.com/denoland/eszip
    let js_code = include_str!("./runtime/example.js");
    
    let req = HttpRequest {
        method: "GET".to_string(),
        url: "/".to_string(),
        headers: HashMap::new(),
        body: None,
    };

    let res = match pool.execute(js_code.to_string(), req).await {
        Ok(r) => r,
        Err(e) => {
            log::error!("Handler invocation failed: {}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR, "Internal Server Error").into_response();
        }
    };

    let mut builder = Response::builder().status(res.status);
    for (key, value) in res.headers.iter() {
        builder = builder.header(key, value);
    }

    builder.body(Body::from(res.body)).unwrap()
}

pub fn build(pool: Arc<WorkerPool>) -> Router {
    Router::new()
        .route("/_health", get(health))
        .route("/", get(invoke))
        .with_state(pool)
}