use anyhow::Result;
use axum::{
    Router,
    body::Body,
    extract::State,
    http::{Request, StatusCode},
    response::{IntoResponse, Response},
};
use hyper_util::{
    // https://github.com/hyperium/hyper/issues/3948
    client::legacy::{Client, connect::HttpConnector},
    rt::TokioExecutor,
};
use std::sync::Arc;
use tracing::{error, info};

#[derive(Clone)]
struct ProxyState {
    client: Client<HttpConnector, Body>,
    proxy_port: u16,
}

pub async fn serve(addr: &str, proxy_port: u16) -> Result<()> {
    let client = Client::builder(TokioExecutor::new()).build_http();
    let state = Arc::new(ProxyState { client, proxy_port });
    let router = Router::new().fallback(proxy_handler).with_state(state);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    info!(component = "http", addr = %addr, proxy_port, "listening");
    axum::serve(listener, router).await?;
    Ok(())
}

async fn proxy_handler(
    State(state): State<Arc<ProxyState>>,
    mut req: Request<Body>,
) -> Result<Response, RoutingError> {
    let proxy_uri = format!(
        "http://127.0.0.1:{}{}",
        state.proxy_port,
        req.uri()
            .path_and_query()
            .map(|pq| pq.as_str())
            .unwrap_or("/")
    );

    info!(
        method = %req.method(),
        uri = %req.uri(),
        proxy_uri = %proxy_uri,
        "proxying request"
    );

    *req.uri_mut() = proxy_uri.parse().map_err(|e| {
        error!("failed to parse proxy URI: {}", e);
        RoutingError::BadRequest
    })?;

    let response = state.client.request(req).await.map_err(|e| {
        error!("proxy request failed: {}", e);
        RoutingError::ProxyError
    })?;

    Ok(response.map(Body::new))
}

enum RoutingError {
    BadRequest,
    ProxyError,
}

impl IntoResponse for RoutingError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            RoutingError::BadRequest => (StatusCode::BAD_REQUEST, "Bad request"),
            RoutingError::ProxyError => (StatusCode::BAD_GATEWAY, "Proxy error"),
        };
        (status, message).into_response()
    }
}
