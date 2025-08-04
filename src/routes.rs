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
    let js_code = r#"
        async function handler(request) {
            // Simulate async operation (e.g., database call, API request)
            await new Promise(resolve => setTimeout(resolve, 1000));
            
            return {
                status: 200,
                headers: { "content-type": "text/html" },
                body: `<h1>Async JavaScript Handler Response</h1>
                       <p>Request method: ${request.method}</p>
                       <p>Request path: ${request.url}</p>
                       <p>Current time: ${new Date().toISOString()}</p>
                       <p>Handler executed asynchronously!</p>`
            };
        }
    "#;
    
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