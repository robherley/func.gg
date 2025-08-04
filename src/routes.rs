use std::collections::HashMap;

use axum::{body::Body, http::StatusCode, response::{Html, IntoResponse, Response}, routing::get, Router};
use crate::runtime::{HttpRequest, JavaScriptRuntime};

pub async fn health() -> Html<&'static str> {
    Html("OK")
}

pub async fn invoke() -> Response {
    let js_code = r#"
        function handler(request) {
            return {
                status: 200,
                headers: { "content-type": "text/html" },
                body: `<h1>JavaScript Handler Response</h1>
                       <p>Request method: ${request.method}</p>
                       <p>Request path: ${request.path}</p>
                       <p>Current time: ${new Date().toISOString()}</p>`
            };
        }
    "#;

    let mut runtime = match JavaScriptRuntime::new() {
        Ok(rt) => rt,
        Err(e) => {
            log::error!("Failed to create JavaScript runtime: {}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR, "Internal Server Error").into_response();
        }
    };

    // todo: handle init errors
    runtime.load_handler(js_code.into());
    
    let req = HttpRequest{
        method: "GET".to_string(),
        url: "/".to_string(),
        headers: HashMap::new(),
        body: None,
    };

    let res = match runtime.invoke_handler(req).await {
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

pub fn build() -> Router {
    Router::new()
        .route("/_health", get(health))
        .route("/", get(invoke))
}