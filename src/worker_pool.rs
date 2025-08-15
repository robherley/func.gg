use anyhow::Result;
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
    receiver: mpsc::UnboundedReceiver<WorkerRequest>,
    response_sender: mpsc::UnboundedSender<WorkerResponse>,
}

impl Worker {
    fn new(
        id: usize,
        receiver: mpsc::UnboundedReceiver<WorkerRequest>,
        response_sender: mpsc::UnboundedSender<WorkerResponse>,
    ) -> Self {
        Self {
            id,
            receiver,
            response_sender,
        }
    }

    async fn run(&mut self) {
        log::info!(worker_id = self.id; "Worker starting");
        
        let mut runtime = match JavaScriptRuntime::new() {
            Ok(rt) => rt,
            Err(e) => {
                log::error!(
                    worker_id = self.id,
                    error:? = e;
                    "Failed to create JavaScript runtime"
                );
                return;
            }
        };

        while let Some(request) = self.receiver.recv().await {
            log::info!(
                worker_id = self.id,
                request_id:? = request.id;
                "Worker accepted request"
            );
            
            let result = self.process_request(&mut runtime, request.clone()).await;
            
            let response = WorkerResponse {
                id: request.id,
                result,
            };

            if let Err(e) = self.response_sender.send(response) {
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

    async fn process_request(
        &self,
        runtime: &mut JavaScriptRuntime,
        request: WorkerRequest,
    ) -> Result<HttpResponse, String> {
        // if let Err(e) = runtime.load_handler(request.js_code).await {
        //     return Err(format!("Failed to load handler: {}", e));
        // }

        match runtime.invoke_handler(request.http_request).await {
            Ok(response) => Ok(response),
            Err(e) => Err(format!("Handler invocation failed: {}", e)),
        }
    }
}

pub struct WorkerPool {
    work_senders: Vec<mpsc::UnboundedSender<WorkerRequest>>,
    response_receiver: Arc<Mutex<mpsc::UnboundedReceiver<WorkerResponse>>>,
    pending_requests: Arc<Mutex<HashMap<Uuid, oneshot::Sender<Result<HttpResponse, String>>>>>,
    current_worker: Arc<Mutex<usize>>,
    pool_size: usize,
}

impl WorkerPool {
    pub fn new(pool_size: usize) -> Self {
        let (response_sender, response_receiver) = mpsc::unbounded_channel();
        let mut work_senders = Vec::with_capacity(pool_size);
        
        // TODO: should init JsRuntime in main thread??
        // https://docs.rs/deno_core/0.353.0/deno_core/struct.JsRuntime.html#method.init_platform

        for i in 0..pool_size {
            let (work_sender, work_receiver) = mpsc::unbounded_channel();
            let response_sender = response_sender.clone();
            
            work_senders.push(work_sender);
            
            // each runtime gets its own thread
            std::thread::spawn(move || {
                let rt = tokio::runtime::Runtime::new().unwrap();
                rt.block_on(async {
                    let mut worker = Worker::new(i, work_receiver, response_sender);
                    worker.run().await;
                });
            });
        }

        let pool = Self {
            work_senders,
            response_receiver: Arc::new(Mutex::new(response_receiver)),
            pending_requests: Arc::new(Mutex::new(HashMap::new())),
            current_worker: Arc::new(Mutex::new(0)),
            pool_size,
        };

        pool.spawn_handler();
        
        pool
    }

    fn spawn_handler(&self) {
        let response_receiver = self.response_receiver.clone();
        let pending_requests = self.pending_requests.clone();
        
        tokio::spawn(async move {
            let mut receiver = response_receiver.lock().await;
            
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

    pub async fn execute(
        &self,
        js_code: String,
        http_request: HttpRequest,
    ) -> Result<HttpResponse, String> {
        let request_id = Uuid::now_v7();
        let (response_sender, response_receiver) = oneshot::channel();

        let pending_count = {
            let pending = self.pending_requests.lock().await;
            pending.len()
        };
        
        if pending_count >= self.pool_size {
            // our throughput sucks?
            log::warn!(
                pending_requests = pending_count + 1,
                available_workers = self.pool_size;
                "Queue overflow detected"
            );
        }

        {
            let mut pending = self.pending_requests.lock().await;
            pending.insert(request_id, response_sender);
        }

        // TODO: is round robin good enough
        let worker_index = {
            let mut current = self.current_worker.lock().await;
            let index = *current;
            *current = (*current + 1) % self.work_senders.len();
            index
        };

        let request = WorkerRequest {
            id: request_id,
            js_code,
            http_request,
        };

        log::debug!(
            request_id:? = request_id,
            worker_index = worker_index;
            "Dispatching request to worker"
        );

        if let Err(e) = self.work_senders[worker_index].send(request) {
            let mut pending = self.pending_requests.lock().await;
            pending.remove(&request_id);
            return Err(format!("Failed to send work to worker: {}", e));
        }

        // TODO: what if we want to **stream** the response?
        match response_receiver.await {
            Ok(result) => result,
            Err(_) => Err("Response channel closed".to_string()),
        }
    }
}