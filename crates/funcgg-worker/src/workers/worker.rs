use serde::{Deserialize, Serialize};
use std::time::Duration;
use tokio::sync::mpsc;
use uuid::Uuid;

use super::pool::StateChange;
use funcgg_runtime::{Sandbox, http};

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
    pub supervisor_tx: mpsc::UnboundedSender<StateChange>,
    pub timeout: Duration,
}

impl Worker {
    pub fn new(
        id: usize,
        request_rx: mpsc::UnboundedReceiver<WorkerRequest>,
        supervisor_tx: mpsc::UnboundedSender<StateChange>,
    ) -> Self {
        Self {
            id,
            request_rx,
            supervisor_tx,
            timeout: Duration::from_secs(30),
        }
    }

    pub async fn run(&mut self) {
        tracing::info!("Worker starting");

        while let Some(request) = self.request_rx.recv().await {
            let request_id = request.id;
            self.notify(StateChange::Received(self.id, self.timeout));
            tracing::info!("Worker accepted request");

            let result = self.process_request(request).await;
            self.notify(StateChange::Finished(self.id, request_id, result));
        }

        tracing::info!("Worker shutting down");
    }

    async fn process_request(&self, request: WorkerRequest) -> Result<http::Response, String> {
        let mut sandbox = match Sandbox::new(request.id) {
            Ok(rt) => rt,
            Err(e) => {
                tracing::error!(
                    error = %e,
                    "Failed to create JavaScript runtime"
                );
                return Err(format!("unable to create runtime: {}", e));
            }
        };
        self.notify(StateChange::Initialized(self.id));

        let handle = sandbox.runtime.v8_isolate().thread_safe_handle();
        self.notify(StateChange::Started(self.id, handle));

        sandbox
            .execute(request.js_code, request.http_request, self.timeout)
            .await
            .map_err(|e| format!("handler invocation failed: {}", e))
    }

    fn notify(&self, msg: StateChange) {
        if let Err(e) = self.supervisor_tx.send(msg) {
            tracing::error!(
                error = %e,
                "Failed to notify supervisor: {e}"
            );
        }
    }
}
