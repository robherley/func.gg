use anyhow::Result;
use deno_core::{JsRuntime, v8::IsolateHandle};
use rand::Rng;
use rand::seq::SliceRandom;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{Mutex, mpsc, oneshot};
use tokio::time::sleep;
use uuid::Uuid;

use super::worker::{Worker, WorkerRequest, WorkerResponse};
use crate::runtime::http;

#[derive(Debug)]
pub enum SupervisorMessage {
    Store(usize, IsolateHandle, Duration),
    Release(usize),
}

struct WorkingWorker {
    id: usize,
    handle: IsolateHandle,
    deadline: tokio::time::Instant,
}

pub struct Pool {
    // Senders for worker requests
    worker_txs: Vec<mpsc::UnboundedSender<WorkerRequest>>,
    // Receiver for worker responses
    responder_rx: Arc<Mutex<mpsc::UnboundedReceiver<WorkerResponse>>>,
    // Receiver for worker events
    supervisor_rx: Arc<Mutex<mpsc::UnboundedReceiver<SupervisorMessage>>>,
    // Requests waiting for workers to do work
    pending_requests: Arc<Mutex<HashMap<Uuid, oneshot::Sender<Result<http::Response, String>>>>>,
    // Active workers with attributes and isolate reference
    current_workers: Arc<Mutex<HashMap<usize, WorkingWorker>>>,
    // The size of the worker pool
    pool_size: usize,
}

impl Pool {
    pub fn new(pool_size: usize) -> Self {
        let (responder_tx, responder_rx) = mpsc::unbounded_channel();
        let (supervisor_tx, supervisor_rx) = mpsc::unbounded_channel();
        let worker_txs = Vec::with_capacity(pool_size);

        let mut pool = Self {
            pool_size,
            worker_txs,
            responder_rx: Arc::new(Mutex::new(responder_rx)),
            supervisor_rx: Arc::new(Mutex::new(supervisor_rx)),
            pending_requests: Arc::new(Mutex::new(HashMap::new())),
            current_workers: Arc::new(Mutex::new(HashMap::new())),
        };

        pool.spawn_reciever();
        pool.spawn_supervisor();
        pool.spawn_workers(responder_tx, supervisor_tx);
        pool
    }

    pub async fn handle(
        &self,
        js_code: String,
        http_request: http::Request,
    ) -> Result<http::Response, String> {
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

    fn spawn_workers(
        &mut self,
        responder_tx: mpsc::UnboundedSender<WorkerResponse>,
        supervisor_tx: mpsc::UnboundedSender<SupervisorMessage>,
    ) {
        // https://docs.rs/deno_core/0.353.0/deno_core/struct.JsRuntime.html#method.init_platform
        JsRuntime::init_platform(Default::default(), false);

        for i in 0..self.pool_size {
            let (request_tx, request_rx) = mpsc::unbounded_channel();
            self.worker_txs.push(request_tx);

            let responder_tx = responder_tx.clone();
            let supervisor_tx = supervisor_tx.clone();

            // each runtime needs its own thread, specifically with tokio's current thread runtime
            std::thread::spawn(move || {
                let rt = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .unwrap();
                rt.block_on(async {
                    let mut worker = Worker::new(i, request_rx, responder_tx, supervisor_tx);
                    worker.run().await;
                });
            });
        }
    }

    fn spawn_supervisor(&self) {
        let supervisor_rx = self.supervisor_rx.clone();
        let current_workers = self.current_workers.clone();
        tokio::spawn(async move {
            let mut receiver = supervisor_rx.lock().await;

            loop {
                tokio::select! {
                    supervisor_msg = receiver.recv() => {
                        match supervisor_msg {
                            Some(SupervisorMessage::Store(worker_id, isolate_handle, duration)) => {
                                let mut current_workers = current_workers.lock().await;
                                log::info!(worker_id = worker_id; "Supervisor received isolate handle");
                                current_workers.insert(worker_id, WorkingWorker {
                                    id: worker_id,
                                    handle: isolate_handle,
                                    deadline: tokio::time::Instant::now() + duration,
                                });
                            }
                            Some(SupervisorMessage::Release(worker_id)) => {
                                let mut current_workers = current_workers.lock().await;
                                log::info!(worker_id = worker_id; "Supervisor received release message");
                                _ = current_workers.remove(&worker_id)
                            }
                            None => {
                                log::info!("Supervisor channel closed");
                                break;
                            }
                        }
                    }

                    _ = sleep(Duration::from_millis(100)) => {
                        let now = tokio::time::Instant::now();
                        let mut to_remove = Vec::new();
                        let mut current_workers = current_workers.lock().await;

                        for (worker_id, worker) in current_workers.iter() {
                            if now > worker.deadline {
                                to_remove.push(*worker_id);
                            }
                        }

                        for worker_id in to_remove {
                            if let Some(worker) = current_workers.remove(&worker_id) {
                                log::warn!(worker_id = worker.id; "Supervisor terminating isolate (worker_id={}) after deadline", worker.id);
                                worker.handle.terminate_execution();
                            }
                        }
                    }
                }
            }
        });
    }

    fn spawn_reciever(&self) {
        let responder_rx = self.responder_rx.clone();
        let pending_requests = self.pending_requests.clone();
        tokio::spawn(async move {
            let mut receiver = responder_rx.lock().await;
            while let Some(response) = receiver.recv().await {
                let mut pending = pending_requests.lock().await;
                if let Some(sender) = pending.remove(&response.id) {
                    if let Err(Err(err)) = sender.send(response.result) {
                        log::warn!(request_id:? = response.id; "Failed to send response to waiting request: {}", err);
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
        response_tx: oneshot::Sender<Result<http::Response, String>>,
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

    /// Finds the next free worker index. If all workers are busy, it picks the one with the lowest deadline.
    /// TODO: this does not currently take into account the initialization time of the sandbox, just executing the modules.
    async fn next_worker_idx(&self) -> usize {
        let mut worker_ids: Vec<usize> = (0..self.pool_size).collect();
        worker_ids.shuffle(&mut rand::rng());
        let current_workers = self.current_workers.lock().await;

        let mut candidate = None;
        for worker_id in worker_ids {
            match current_workers.get(&worker_id) {
                Some(worker) => match candidate {
                    Some((_, min)) => {
                        if worker.deadline < min {
                            candidate = Some((worker_id, worker.deadline));
                        }
                    }
                    None => candidate = Some((worker_id, worker.deadline)),
                },
                None => return worker_id,
            }
        }

        match candidate {
            Some((worker_id, _)) => worker_id,
            None => rand::rng().random_range(0..self.pool_size),
        }
    }
}
