use axum::{
    Router,
    body::Body,
    extract::{Request, State},
    http::{StatusCode, Uri},
    response::{IntoResponse, Response},
};
use deno_core::futures::StreamExt;
use funcgg_runtime::comms;
use std::sync::Arc;
use std::{collections::HashMap, convert::Infallible};
use tokio::sync::{mpsc, oneshot};
use tokio_stream::wrappers::ReceiverStream;

use crate::pool::Pool;

pub async fn invoke(State(pool): State<Arc<Pool>>, request: Request) -> Response {
    // TODO: temporary, eventually this should be served dynamically
    // possibly serve https://github.com/denoland/eszip
    let js_code = match tokio::fs::read_to_string("crates/funcgg-worker/examples/basic.js").await {
        Ok(code) => code,
        Err(err) => {
            tracing::error!("Failed to load JavaScript code: {}", err);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to load handler code",
            )
                .into_response();
        }
    };

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

    let req = funcgg_runtime::comms::Request {
        method,
        uri,
        headers,
    };

    let (response_body_tx, response_body_rx) = mpsc::channel::<bytes::Bytes>(1);
    let (response_tx, response_rx) = oneshot::channel::<comms::Response>();

    let channels = comms::Channels {
        incoming_body_rx: body_rx,
        outgoing_body_tx: response_body_tx,
        response_tx,
    };

    if let Err(err) = pool.send_work(js_code.to_string(), req, channels).await {
        tracing::error!("Handler invocation failed: {}", err);
        return (StatusCode::INTERNAL_SERVER_ERROR, "Internal Server Error").into_response();
    };

    let res = match response_rx.await {
        Ok(res) => res,
        Err(err) => {
            tracing::error!("Response channel error: {}", err);
            return (StatusCode::INTERNAL_SERVER_ERROR, "Internal Server Error").into_response();
        }
    };

    let mut builder = Response::builder().status(res.status);
    for (key, value) in res.headers.iter() {
        builder = builder.header(key, value);
    }

    let stream = ReceiverStream::new(response_body_rx).map(Ok::<_, Infallible>);
    match builder.body(Body::from_stream(stream)) {
        Ok(response) => response,
        Err(err) => {
            tracing::error!("Failed to build response: {}", err);
            (StatusCode::INTERNAL_SERVER_ERROR, "Internal Server Error").into_response()
        }
    }
}

pub fn build(pool: Arc<Pool>) -> Router {
    Router::new().fallback(invoke).with_state(pool)
}
