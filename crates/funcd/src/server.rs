use anyhow::Result;
use http_body_util::BodyExt;

pub struct Proxy {
    client: reqwest::Client,
    pub upstream: String,
}

impl Proxy {
    pub fn new(host: String, port: u16) -> Self {
        Self {
            upstream: format!("http://{}:{}", host, port),
            client: reqwest::Client::new(),
        }
    }

    pub async fn handle(
        &self,
        req: http::Request<lambda_http::Body>,
    ) -> Result<http::Response<lambda_http::Body>, lambda_http::Error> {
        let (parts, body) = req.into_parts();

        let body_bytes = body
            .collect()
            .await
            .map_err(|e| lambda_http::Error::from(format!("unable to read request body: {}", e)))?
            .to_bytes();

        let upstream_url = format!(
            "{}{}",
            self.upstream,
            parts
                .uri
                .path_and_query()
                .map(|pq| pq.as_str())
                .unwrap_or("/")
        );

        let mut upstream_req = self
            .client
            .request(parts.method, &upstream_url)
            .body(body_bytes);

        for (name, value) in parts.headers.iter() {
            if include_header(&name) {
                upstream_req = upstream_req.header(name, value);
            }
        }

        let upstream_resp = upstream_req
            .send()
            .await
            .map_err(|e| lambda_http::Error::from(format!("upstream request failed: {}", e)))?;

        let mut response_builder = http::Response::builder().status(upstream_resp.status());

        for (name, value) in upstream_resp.headers().iter() {
            response_builder = response_builder.header(name, value);
        }

        let body_bytes = upstream_resp.bytes().await.map_err(|e| {
            lambda_http::Error::from(format!("failed to read response body: {}", e))
        })?;

        let response = response_builder
            .body(lambda_http::Body::from(body_bytes.as_ref()))
            .map_err(|e| lambda_http::Error::from(e))?;

        Ok(response)
    }
}

fn include_header(name: &http::HeaderName) -> bool {
    let name_str = name.as_str();
    match name {
        &http::header::HOST => false,
        _ if name_str.starts_with("x-amzn") => false,
        _ => true,
    }
}
