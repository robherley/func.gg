use anyhow::{Result, anyhow};
use deno_core::url::Url;
use deno_core::{JsRuntime, RuntimeOptions, v8};
use rustls::crypto::{CryptoProvider, aws_lc_rs};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::LazyLock;
use std::time::Duration;
use tokio::sync::{Mutex, mpsc, oneshot};
use uuid::Uuid;

use super::comms;
use super::ext;
use super::loader;

static HEAP_LIMIT: usize = 64 * 1024 * 1024; // 64MB

static WORKER_CODE: &str = include_str!("./js/worker.js");
static WORKER_MOD_SPECIFIER: LazyLock<Url> =
    LazyLock::new(|| "func:worker".parse().expect("bad module specifier"));

static USER_MOD_SPECIFIER: LazyLock<Url> =
    LazyLock::new(|| "func:user-code".parse().expect("bad module specifier"));

pub struct State {
    pub req: Option<comms::Request>,
    pub res: Option<comms::Response>,
    pub request_id: Uuid,
    pub incoming_body_rx: Rc<Mutex<mpsc::Receiver<Result<bytes::Bytes, String>>>>,
    pub outgoing_body_tx: mpsc::Sender<bytes::Bytes>,
    pub response_tx: Option<oneshot::Sender<comms::Response>>,
}

pub struct Sandbox {
    pub runtime: JsRuntime,
    pub state: Rc<RefCell<State>>,
}

impl Sandbox {
    pub fn new(
        request_id: Uuid,
        startup_snapshot: Option<&'static [u8]>,
        channels: comms::Channels,
    ) -> Result<Self> {
        _ = CryptoProvider::install_default(aws_lc_rs::default_provider());

        let state = Rc::new(RefCell::new(State {
            request_id,
            req: None,
            res: None,
            incoming_body_rx: Rc::new(Mutex::new(channels.incoming_body_rx)),
            outgoing_body_tx: channels.outgoing_body_tx,
            response_tx: Some(channels.response_tx),
        }));

        let extension_transpiler = Rc::new(loader::transpile);
        let create_params = v8::CreateParams::default().heap_limits(0, HEAP_LIMIT);

        let mod_loader = Rc::new(loader::ModuleLoader::new());
        let mut runtime = JsRuntime::try_new(RuntimeOptions {
            module_loader: Some(mod_loader),
            extensions: ext::extensions(),
            extension_transpiler: Some(extension_transpiler),
            create_params: Some(create_params),
            startup_snapshot,
            ..Default::default()
        })?;

        let handle = runtime.v8_isolate().thread_safe_handle();
        runtime.add_near_heap_limit_callback(move |heap_size, _| {
            tracing::warn!("heap size exceeded ({}), terminating...", heap_size);

            handle.terminate_execution();
            // give it some extra room to clean up w/o crashing the runtime
            // TODO: some way to notify that this request exceeded the heap limit
            heap_size * 4
        });

        runtime.op_state().borrow_mut().put(state.clone());

        Ok(Self { runtime, state })
    }

    pub async fn execute(
        &mut self,
        user_code: String,
        request: comms::Request,
        timeout_duration: Duration,
    ) -> Result<()> {
        let execution_result = tokio::time::timeout(timeout_duration, async {
            let _ = self
                .runtime
                .load_side_es_module_from_code(&USER_MOD_SPECIFIER, user_code)
                .await?;
            let entrypoint_id = self
                .runtime
                .load_main_es_module_from_code(&WORKER_MOD_SPECIFIER, WORKER_CODE)
                .await?;

            self.state.borrow_mut().req = Some(request);
            let result = self.runtime.mod_evaluate(entrypoint_id);
            self.runtime.run_event_loop(Default::default()).await?;
            result.await?;
            Ok::<(), anyhow::Error>(())
        })
        .await;

        match execution_result {
            Ok(Ok(())) => {
                // completed
            }
            Ok(Err(e)) => {
                // an error occurred
                return Err(e);
            }
            Err(_) => {
                // timeout occurred
                return Err(anyhow!("JavaScript execution timed out"));
            }
        }

        Ok(())
    }
}
