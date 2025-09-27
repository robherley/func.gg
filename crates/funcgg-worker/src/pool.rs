use anyhow::Result;
use deno_core::{JsRuntime, v8::IsolateHandle};
use rand::Rng;
use rand::seq::SliceRandom;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{Mutex, mpsc, oneshot};
use tokio::time::{Instant, sleep};
use uuid::Uuid;

use crate::worker::{Worker, WorkerRequest};
use funcgg_runtime::http;

type PendingRequests = HashMap<Uuid, oneshot::Sender<Result<http::Response, String>>>;

pub enum StateChange {
    Received(usize, Duration),
    Initialized(usize),
    Started(usize, IsolateHandle),
    Finished(usize, Uuid, Result<http::Response, String>),
}

impl std::fmt::Debug for StateChange {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StateChange::Received(worker_id, _) => {
                write!(f, "Received({})", worker_id)
            }
            StateChange::Initialized(worker_id) => {
                write!(f, "Initialized({})", worker_id)
            }
            StateChange::Started(worker_id, _) => {
                write!(f, "Started({})", worker_id)
            }
            StateChange::Finished(worker_id, _, _) => {
                write!(f, "Finished({})", worker_id)
            }
        }
    }
}

pub struct WorkerState {
    deadline: Instant,
    isolate: Option<IsolateHandle>,
    timings: HashMap<String, Duration>,
    last_checkpoint: Option<(String, Instant)>,
}

impl WorkerState {
    pub fn new(deadline: Instant) -> Self {
        Self {
            deadline,
            isolate: None,
            timings: HashMap::new(),
            last_checkpoint: None,
        }
    }

    pub fn checkpoint(&mut self, name: &str) {
        if let Some((chk_name, chk_time)) = self.last_checkpoint.take() {
            self.timings
                .insert(format!("{} to {}", chk_name, name), chk_time.elapsed());
        }
        self.last_checkpoint = Some((name.to_string(), Instant::now()));
    }
}

pub struct Pool {
    // Senders for worker requests
    worker_txs: Vec<mpsc::UnboundedSender<WorkerRequest>>,
    // Receiver for worker events
    supervisor_rx: Arc<Mutex<mpsc::UnboundedReceiver<StateChange>>>,
    // Requests waiting for workers to do work
    pending_requests: Arc<Mutex<PendingRequests>>,
    // Active workers with attributes and isolate reference
    current_workers: Arc<Mutex<HashMap<usize, WorkerState>>>,
    // The size of the worker pool
    pool_size: usize,
    // The address of the worker pool
    pub addr: String,
}

impl Pool {
    pub fn new(pool_size: usize, addr: String) -> Self {
        let (supervisor_tx, supervisor_rx) = mpsc::unbounded_channel();
        let worker_txs = Vec::with_capacity(pool_size);

        let mut pool = Self {
            pool_size,
            worker_txs,
            supervisor_rx: Arc::new(Mutex::new(supervisor_rx)),
            pending_requests: Arc::new(Mutex::new(HashMap::new())),
            current_workers: Arc::new(Mutex::new(HashMap::new())),
            addr,
        };

        pool.spawn_supervisor();
        pool.spawn_workers(supervisor_tx);
        pool
    }

    pub async fn handle(
        &self,
        js_code: String,
        http_request: http::Request,
        incoming_body_rx: mpsc::Receiver<Result<bytes::Bytes, String>>,
    ) -> Result<http::Response, String> {
        let request_id = Uuid::now_v7();
        let (response_tx, response_rx) = oneshot::channel();

        let worker_idx = self.next_worker_idx().await;
        let request = WorkerRequest {
            id: request_id,
            js_code,
            http_request,
            incoming_body_rx,
        };

        self.insert_pending(request_id, response_tx).await;

        tracing::debug!(
            request_id = ?request_id,
            worker_idx = worker_idx,
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

    fn spawn_workers(&mut self, supervisor_tx: mpsc::UnboundedSender<StateChange>) {
        // https://docs.rs/deno_core/0.353.0/deno_core/struct.JsRuntime.html#method.init_platform
        JsRuntime::init_platform(Default::default(), false);

        for i in 0..self.pool_size {
            let (request_tx, request_rx) = mpsc::unbounded_channel();
            self.worker_txs.push(request_tx);

            let supervisor_tx = supervisor_tx.clone();

            // each runtime needs its own thread, specifically with tokio's current thread runtime
            std::thread::spawn(move || {
                let _span = tracing::info_span!("worker", id = i).entered();
                let rt = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .unwrap();
                rt.block_on(async {
                    let mut worker = Worker::new(i, request_rx, supervisor_tx);
                    worker.run().await;
                });
            });
        }
    }

    fn spawn_supervisor(&self) {
        let supervisor_rx = self.supervisor_rx.clone();
        let current_workers = self.current_workers.clone();
        let pending_requests = self.pending_requests.clone();
        tokio::spawn(async move {
            let mut receiver = supervisor_rx.lock().await;

            loop {
                tokio::select! {
                    msg = receiver.recv() => {
                        tracing::info!("Supervisor received message: {:?}", msg);
                        match msg {
                            Some(StateChange::Received(worker_id, duration)) => {
                                let mut current_workers = current_workers.lock().await;
                                let mut worker = WorkerState::new(Instant::now() + duration);
                                worker.checkpoint("recv");
                                current_workers.insert(worker_id, worker);
                            }
                            Some(StateChange::Initialized(worker_id)) => {
                                let mut current_workers = current_workers.lock().await;
                                if let Some(worker) = current_workers.get_mut(&worker_id) {
                                    worker.checkpoint("init");
                                }
                            }
                            Some(StateChange::Started(worker_id, isolate_handle)) => {
                                let mut current_workers = current_workers.lock().await;
                                if let Some(worker) = current_workers.get_mut(&worker_id) {
                                    worker.checkpoint("start");
                                    worker.isolate = Some(isolate_handle);
                                }
                            }
                            Some(StateChange::Finished(worker_id, request_id, response)) => {
                                let mut current_workers = current_workers.lock().await;
                                if let Some(mut worker) = current_workers.remove(&worker_id) {
                                    worker.checkpoint("finish");
                                    tmp_timings(worker_id, request_id, worker.timings);
                                }

                                let mut pending_requests = pending_requests.lock().await;
                                if let Some(sender) = pending_requests.remove(&request_id) && sender.send(response).is_err() {
                                    tracing::warn!(request_id = ?request_id, "Failed to send response to waiting request");
                                }
                            }
                            None => {
                                tracing::info!("Supervisor channel closed");
                                break;
                            }
                        }
                    }

                    _ = sleep(Duration::from_millis(200)) => {
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
                                tracing::warn!(worker_id = worker_id, "Supervisor terminating isolate after deadline");
                                if let Some(isolate) = worker.isolate {
                                    isolate.terminate_execution();
                                }
                            }
                        }
                    }
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
            tracing::warn!(
                pending_requests = pending.len() + 1,
                available_workers = self.pool_size,
                "queue backup detected: more pending than available"
            );
        }
    }

    /// Finds the next free worker index. If all workers are busy, it picks the one with the lowest deadline.
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

fn tmp_timings(id: usize, request_id: Uuid, timings: HashMap<String, Duration>) {
    tracing::info!("worker {} on request {}: {:?}", id, request_id, timings);
}
