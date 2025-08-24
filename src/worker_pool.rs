use anyhow::Result;
use deno_core::JsRuntime;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, oneshot, Mutex};
use uuid::Uuid;

use crate::runtime::{HttpRequest, HttpResponse, JavaScriptRuntime};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerRequest {
    pub id: Uuid,
    pub js_code: String,
    pub http_request: HttpRequest,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerResponse {
    pub id: Uuid,
    pub result: Result<HttpResponse, String>,
}

struct Worker {
    id: usize,
    request_rx: mpsc::UnboundedReceiver<WorkerRequest>,
    responder_tx: mpsc::UnboundedSender<WorkerResponse>,
}

impl Worker {
    fn new(
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

    async fn run(&mut self) {
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

    async fn process_request(&self, request: WorkerRequest) -> Result<HttpResponse, String> {
        let mut runtime = match JavaScriptRuntime::new(request.id) {
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

pub struct WorkerPool {
    worker_txs: Vec<mpsc::UnboundedSender<WorkerRequest>>,
    responder_rx: Arc<Mutex<mpsc::UnboundedReceiver<WorkerResponse>>>,
    pending_requests: Arc<Mutex<HashMap<Uuid, oneshot::Sender<Result<HttpResponse, String>>>>>,
    current_worker_idx: Arc<Mutex<usize>>,
    pool_size: usize,
}

impl WorkerPool {
    pub fn new(pool_size: usize) -> Self {
        let (responder_tx, responder_rx) = mpsc::unbounded_channel();
        let worker_txs = Vec::with_capacity(pool_size);

        let mut pool = Self {
            pool_size,
            worker_txs,
            responder_rx: Arc::new(Mutex::new(responder_rx)),
            pending_requests: Arc::new(Mutex::new(HashMap::new())),
            current_worker_idx: Arc::new(Mutex::new(0)),
        };

        pool.spawn_reciever();
        pool.spawn_workers(responder_tx);
        pool
    }

    pub async fn handle(
        &self,
        js_code: String,
        http_request: HttpRequest,
    ) -> Result<HttpResponse, String> {
        let request_id = Uuid::now_v7();
        let (response_tx, response_rx) = oneshot::channel();

        let worker_idx = self.next_worker_idx().await;
        let request = WorkerRequest {
            id: request_id,
            js_code,
            http_request,
        };

        self.insert_pending(request_id, response_tx).await;

        log::debug!(
            request_id:? = request_id,
            worker_idx = worker_idx;
            "Dispatching request to worker"
        );

        if let Err(e) = self.worker_txs[worker_idx].send(request) {
            let mut pending = self.pending_requests.lock().await;
            pending.remove(&request_id);
            return Err(format!("failed to send work to worker: {}", e));
        }

        // TODO: what if we want to **stream** the response?
        match response_rx.await {
            Ok(result) => result,
            Err(e) => Err(format!("unable to receive: {}", e)),
        }
    }

    fn spawn_workers(&mut self, responder_tx: mpsc::UnboundedSender<WorkerResponse>) {
        // https://docs.rs/deno_core/0.353.0/deno_core/struct.JsRuntime.html#method.init_platform
        JsRuntime::init_platform(Default::default(), false);

        for i in 0..self.pool_size {
            let (request_tx, request_rx) = mpsc::unbounded_channel();
            self.worker_txs.push(request_tx);

            let responder_tx = responder_tx.clone();

            // each runtime needs its own thread
            std::thread::spawn(move || {
                let rt = tokio::runtime::Runtime::new().unwrap();
                rt.block_on(async {
                    let mut worker = Worker::new(i, request_rx, responder_tx);
                    worker.run().await;
                });
            });
        }
    }

    fn spawn_reciever(&self) {
        let responder_rx = self.responder_rx.clone();
        let pending_requests = self.pending_requests.clone();
        tokio::spawn(async move {
            let mut receiver = responder_rx.lock().await;

            while let Some(response) = receiver.recv().await {
                let mut pending = pending_requests.lock().await;

                if let Some(sender) = pending.remove(&response.id) {
                    if let Err(_) = sender.send(response.result) {
                        log::warn!(request_id:? = response.id; "Failed to send response to waiting request");
                    }
                } else {
                    log::warn!(request_id:? = response.id; "Received response for unknown request");
                }
            }
        });
    }

    async fn insert_pending(
        &self,
        request_id: Uuid,
        response_tx: oneshot::Sender<Result<HttpResponse, String>>,
    ) {
        let mut pending = self.pending_requests.lock().await;
        pending.insert(request_id, response_tx);

        if pending.len() >= self.pool_size {
            // our throughput sucks?
            log::warn!(
                pending_requests = pending.len() + 1,
                available_workers = self.pool_size;
                "queue backup detected: more pending than available"
            );
        }
    }

    // TODO: find next free worker instead of round robin
    async fn next_worker_idx(&self) -> usize {
        let mut current = self.current_worker_idx.lock().await;
        let idx = *current;
        *current = (idx + 1) % self.pool_size;
        idx
    }
}
