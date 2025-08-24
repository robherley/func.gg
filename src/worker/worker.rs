use anyhow::Result;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use uuid::Uuid;

use crate::runtime::{http, Sandbox};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerRequest {
    pub id: Uuid,
    pub js_code: String,
    pub http_request: http::Request,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerResponse {
    pub id: Uuid,
    pub result: Result<http::Response, String>,
}

pub struct Worker {
    pub(crate) id: usize,
    pub(crate) request_rx: mpsc::UnboundedReceiver<WorkerRequest>,
    pub(crate) responder_tx: mpsc::UnboundedSender<WorkerResponse>,
}

impl Worker {
    pub(crate) fn new(
        id: usize,
        request_rx: mpsc::UnboundedReceiver<WorkerRequest>,
        responder_tx: mpsc::UnboundedSender<WorkerResponse>,
    ) -> Self {
        Self {
            id,
            request_rx,
            responder_tx,
        }
    }

    pub(crate) async fn run(&mut self) {
        log::info!(worker_id = self.id; "Worker {} starting", self.id);

        while let Some(request) = self.request_rx.recv().await {
            let request_id = request.id;
            log::info!(
                worker_id = self.id,
                request_id:? = request_id;
                "Worker accepted request"
            );

            let result = self.process_request(request).await;

            let response = WorkerResponse {
                id: request_id,
                result,
            };

            if let Err(e) = self.responder_tx.send(response) {
                log::error!(
                    worker_id = self.id,
                    error:? = e;
                    "Failed to send response"
                );
                break;
            }
        }

        log::info!(worker_id = self.id; "Worker shutting down");
    }

    async fn process_request(&self, request: WorkerRequest) -> Result<http::Response, String> {
        let mut runtime = match Sandbox::new(request.id) {
            Ok(rt) => rt,
            Err(e) => {
                log::error!(
                    worker_id = self.id,
                    error:? = e;
                    "Failed to create JavaScript runtime"
                );
                return Err(format!("failed to create runtime: {}", e));
            }
        };

        match runtime.execute(request.js_code, request.http_request).await {
            Ok(response) => Ok(response),
            Err(e) => Err(format!("handler invocation failed: {}", e)),
        }
    }
}
