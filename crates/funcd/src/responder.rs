use crate::config;
use lambda_http::{Body, Request, Response, service_fn};
use std::sync::Arc;

pub async fn respond(mode: config::Mode, result: anyhow::Result<()>) -> anyhow::Result<()> {
    match mode {
        config::Mode::Local => result,
        config::Mode::Lambda => lambda(result).await,
    }
}

async fn lambda(result: anyhow::Result<()>) -> anyhow::Result<()> {
    let result = Arc::new(result);

    let svc_fn = service_fn(move |_req: Request| {
        let result = Arc::clone(&result);
        async move {
            let response = match result.as_ref() {
                Ok(_) => Response::builder()
                    .status(200)
                    .body(Body::from("OK"))
                    .unwrap(),
                Err(e) => Response::builder()
                    .status(500)
                    .body(Body::from(format!("Invocation failed: {e}")))
                    .unwrap(),
            };
            Ok::<_, lambda_http::Error>(response)
        }
    });

    lambda_http::run(svc_fn)
        .await
        .map_err(|e| anyhow::anyhow!("lambda runtime error: {e}"))
}
