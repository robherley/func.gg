use std::path::PathBuf;

use anyhow::Result;
use http_body_util::BodyExt;
use tokio_stream::StreamExt;
use tracing::error;

pub struct Proxy {
    client: reqwest::Client,
}

impl Proxy {
    pub fn new(upstream: PathBuf) -> Result<Self> {
        let client = reqwest::Client::builder().unix_socket(upstream).build()?;
        Ok(Self { client })
    }

    pub async fn handle_with_streaming_response(
        &self,
        req: http::Request<lambda_http::Body>,
    ) -> Result<http::Response<lambda_runtime::streaming::Body>, lambda_http::Error> {
        let upstream_resp = self.send_upstream_request(req).await?;
        self.build_streaming_response(upstream_resp).await
    }

    pub async fn handle(
        &self,
        req: http::Request<lambda_http::Body>,
    ) -> Result<http::Response<lambda_http::Body>, lambda_http::Error> {
        let upstream_resp = self.send_upstream_request(req).await?;
        self.build_response(upstream_resp).await
    }

    async fn send_upstream_request(
        &self,
        req: http::Request<lambda_http::Body>,
    ) -> Result<reqwest::Response, lambda_http::Error> {
        let upstream_req = self.build_reqwest(req).await?;
        upstream_req
            .send()
            .await
            .map_err(|e| lambda_http::Error::from(format!("upstream request failed: {}", e)))
    }

    async fn build_streaming_response(
        &self,
        upstream_resp: reqwest::Response,
    ) -> Result<http::Response<lambda_runtime::streaming::Body>, lambda_http::Error> {
        let mut response_builder = http::Response::builder().status(upstream_resp.status());

        for (name, value) in upstream_resp.headers().iter() {
            response_builder = response_builder.header(name, value);
        }

        let (mut tx, rx) = lambda_runtime::streaming::channel();
        let mut body_stream = upstream_resp.bytes_stream();
        tokio::spawn(async move {
            while let Some(chunk) = body_stream.next().await {
                match chunk {
                    Ok(chunk) => {
                        if tx.send_data(chunk).await.is_err() {
                            break;
                        }
                    }
                    Err(err) => {
                        error!("upstream response body read failed: {}", err);
                        break;
                    }
                }
            }
        });

        let response = response_builder
            .body(rx)
            .map_err(lambda_http::Error::from)?;

        Ok(response)
    }

    async fn build_response(
        &self,
        upstream_resp: reqwest::Response,
    ) -> Result<http::Response<lambda_http::Body>, lambda_http::Error> {
        let mut response_builder = http::Response::builder().status(upstream_resp.status());

        for (name, value) in upstream_resp.headers().iter() {
            response_builder = response_builder.header(name, value);
        }

        let body_bytes = upstream_resp.bytes().await.map_err(|e| {
            lambda_http::Error::from(format!("failed to read response body: {}", e))
        })?;

        let response = response_builder
            .body(body_bytes.as_ref().into())
            .map_err(lambda_http::Error::from)?;

        Ok(response)
    }

    async fn build_reqwest(
        &self,
        req: http::Request<lambda_http::Body>,
    ) -> Result<reqwest::RequestBuilder, lambda_http::Error> {
        let (parts, body) = req.into_parts();

        let body_bytes = body.collect().await?.to_bytes();

        let host = parts
            .headers
            .get(http::header::HOST)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("func.local");

        let upstream_url = format!(
            "http://{}{}",
            host,
            parts
                .uri
                .path_and_query()
                .map(|pq| pq.as_str())
                .unwrap_or("/")
        );

        let mut upstream_req = self
            .client
            .request(parts.method, upstream_url)
            .body(body_bytes);

        for (name, value) in parts.headers.iter() {
            if include_header(name) {
                upstream_req = upstream_req.header(name, value);
            }
        }

        Ok(upstream_req)
    }
}

fn include_header(name: &http::HeaderName) -> bool {
    match name {
        &http::header::HOST => false,
        _ if name.as_str().starts_with("x-amzn") => false,
        _ => true,
    }
}
