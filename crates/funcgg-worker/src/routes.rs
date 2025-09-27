use axum::{
    Router,
    body::Body,
    extract::{Request, State},
    http::{StatusCode, Uri},
    response::{IntoResponse, Response},
};
use deno_core::futures::StreamExt;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::mpsc;

use crate::pool::Pool;

pub async fn invoke(State(pool): State<Arc<Pool>>, request: Request) -> Response {
    // TODO: temporary, eventually this should be served dynamically
    // possibly serve https://github.com/denoland/eszip
    let js_code = include_str!("../examples/basic.js");

    let method = request.method().to_string();
    let path_and_query = request.uri().path_and_query().unwrap().to_owned();

    let uri = Uri::builder()
        .scheme("http")
        .authority(pool.addr.clone())
        .path_and_query(path_and_query)
        .build()
        .unwrap()
        .to_string();

    let headers = request
        .headers()
        .iter()
        .map(|(name, value)| (name.to_string(), value.to_str().unwrap_or("").to_string()))
        .collect::<HashMap<String, String>>();

    let _content_length = headers
        .get("content-length")
        .and_then(|cl| cl.parse::<usize>().ok())
        .unwrap_or(0);

    let mut stream = request.into_body().into_data_stream();
    let (body_tx, body_rx) = mpsc::channel(1);
    tokio::spawn(async move {
        while let Some(chunk) = stream.next().await {
            let result = chunk.map_err(|err| format!("unable to read request body: {}", err));
            if body_tx.send(result).await.is_err() {
                break;
            }
        }
    });

    let req = funcgg_runtime::http::Request {
        method,
        uri,
        headers,
    };

    let res = match pool.handle(js_code.to_string(), req, body_rx).await {
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

pub fn build(pool: Arc<Pool>) -> Router {
    Router::new().fallback(invoke).with_state(pool)
}
