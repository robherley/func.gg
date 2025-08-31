use serde::{Deserialize, Serialize};
use std::time::Duration;
use tokio::sync::mpsc;
use uuid::Uuid;

use super::pool::SupervisorMessage;
use crate::runtime::{Sandbox, http};

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
    pub id: usize,
    pub request_rx: mpsc::UnboundedReceiver<WorkerRequest>,
    pub responder_tx: mpsc::UnboundedSender<WorkerResponse>,
    pub supervisor_tx: mpsc::UnboundedSender<SupervisorMessage>,
}

impl Worker {
    pub fn new(
        id: usize,
        request_rx: mpsc::UnboundedReceiver<WorkerRequest>,
        responder_tx: mpsc::UnboundedSender<WorkerResponse>,
        supervisor_tx: mpsc::UnboundedSender<SupervisorMessage>,
    ) -> Self {
        Self {
            id,
            request_rx,
            responder_tx,
            supervisor_tx,
        }
    }

    pub async fn run(&mut self) {
        log::info!(worker_id = self.id; "Worker {} starting", self.id);

        while let Some(request) = self.request_rx.recv().await {
            let request_id = request.id;
            log::info!(
                worker_id = self.id,
                request_id:? = request_id;
                "Worker {} accepted request", self.id
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
        // TODO: maybe this can be configurable for users depending on their 'trust' level
        let timeout = Duration::from_secs(30);

        let mut sandbox = match Sandbox::new(request.id) {
            Ok(rt) => rt,
            Err(e) => {
                log::error!(
                    worker_id = self.id,
                    error:? = e;
                    "Failed to create JavaScript runtime"
                );
                return Err(format!("unable to create runtime: {}", e));
            }
        };

        {
            let handle = sandbox.runtime.v8_isolate().thread_safe_handle();
            self.notify_supervisor(SupervisorMessage::Store(self.id, handle, timeout));
        }

        let result = sandbox
            .execute(request.js_code, request.http_request, timeout)
            .await
            .map_err(|e| format!("handler invocation failed: {}", e));

        self.notify_supervisor(SupervisorMessage::Release(self.id));
        result
    }

    fn notify_supervisor(&self, msg: SupervisorMessage) {
        if let Err(e) = self.supervisor_tx.send(msg) {
            log::error!(
                worker_id = self.id,
                error:? = e;
                "Failed to notify supervisor: {e}"
            );
        }
    }
}
