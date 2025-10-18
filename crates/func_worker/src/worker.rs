use std::time::Duration;
use tokio::sync::mpsc;
use uuid::Uuid;

use super::pool::StateChange;
use func_runtime::{Sandbox, comms};

static STARTUP_SNAPSHOT: &[u8] = include_bytes!(env!("SNAPSHOT_PATH"));

#[derive(Debug)]
pub struct WorkerRequest {
    pub id: Uuid,
    pub js_code: String,
    pub http_request: comms::Request,
    pub channels: comms::Channels,
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

            if let Err(err) = self.process_request(request).await {
                // TODO: if a failure happens here, the client might not receive a response
                tracing::error!("Failed to process request: {}", err);
            }
            self.notify(StateChange::Finished(self.id, request_id));
        }

        tracing::info!("Worker shutting down");
    }

    // TODO: make this state change to failure on failure
    async fn process_request(&self, request: WorkerRequest) -> Result<(), String> {
        let mut sandbox = match Sandbox::new(request.id, Some(STARTUP_SNAPSHOT), request.channels) {
            Ok(rt) => rt,
            Err(e) => {
                tracing::error!(
                    error = %e,
                    "Failed to create JavaScript runtime"
                );
                return Err(format!("unable to create runtime: {}", e));
            }
        };

        let handle = sandbox.runtime.v8_isolate().thread_safe_handle();
        self.notify(StateChange::Started(self.id, handle));

        sandbox
            .execute(request.js_code, request.http_request, self.timeout)
            .await
            .map_err(|e| format!("handler invocation failed: {}", e))?;

        Ok(())
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
