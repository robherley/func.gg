use axum::{
    Router,
    body::{Body, to_bytes},
    extract::{Request, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use std::collections::HashMap;
use std::sync::Arc;

use crate::workers;

pub async fn invoke(State(pool): State<Arc<workers::Pool>>, request: Request) -> Response {
    // TODO: temporary, eventually this should be served dynamically
    // possibly serve https://github.com/denoland/eszip
    let js_code = include_str!("../examples/basic.js");

    let method = request.method().to_string();
    let uri = request.uri().to_string();
    let headers = request
        .headers()
        .iter()
        .map(|(name, value)| (name.to_string(), value.to_str().unwrap_or("").to_string()))
        .collect::<HashMap<String, String>>(); // TODO: any nonsense we should filter out?

    // TODO: this should be streamed on both ends, as raw bytes not utf-8
    let body = match to_bytes(request.into_body(), 1024 * 1024).await {
        Ok(bytes) => {
            if bytes.is_empty() {
                None
            } else {
                match String::from_utf8(bytes.to_vec()) {
                    Ok(body_string) => Some(body_string),
                    Err(_) => {
                        tracing::warn!("request body contains invalid UTF-8, treating as empty");
                        None
                    }
                }
            }
        }
        Err(e) => {
            tracing::error!("failed to read request body: {}", e);
            return (StatusCode::BAD_REQUEST, "Unable to read request body").into_response();
        }
    };

    let req = funcgg_runtime::http::Request {
        method,
        uri,
        headers,
        body,
    };

    let res = match pool.handle(js_code.to_string(), req).await {
        Ok(r) => r,
        Err(e) => {
            tracing::error!("Handler invocation failed: {}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR, "Internal Server Error").into_response();
        }
    };

    let mut builder = Response::builder().status(res.status);
    for (key, value) in res.headers.iter() {
        builder = builder.header(key, value);
    }

    builder.body(Body::from(res.body)).unwrap()
}

pub fn build(pool: Arc<workers::Pool>) -> Router {
    Router::new().fallback(invoke).with_state(pool)
}
