use std::sync::{Arc, Mutex};

use anyhow::Result;
use log::{info, warn};
use tokio::spawn;
use wasmtime::component::{Component, Linker};
use wasmtime::{Engine, Store};
use wasmtime_wasi::bindings::Command;
use wasmtime_wasi::*;

use crate::streams::{InputStream, OutputStream};
use crate::wit;

const MAX_RUNTIME_DURATION: std::time::Duration = std::time::Duration::from_secs(30);

// TODO(robherley): adjust config for sandboxing
// ResourceLimiter: https://docs.rs/wasmtime/latest/wasmtime/trait.ResourceLimiter.html
// limiter: https://docs.rs/wasmtime/latest/wasmtime/struct.Store.html#method.limiter
// epoch_interruption: https://docs.rs/wasmtime/latest/wasmtime/struct.Config.html#method.epoch_interruption
// fuel: https://docs.rs/wasmtime/latest/wasmtime/struct.Config.html#method.consume_fuel

#[derive(Debug, Clone)]
pub struct HTTPHead {
    pub status: u16,
    pub headers: Vec<(String, String)>,
}

impl From<HTTPHead> for actix_web::HttpResponseBuilder {
    fn from(head: HTTPHead) -> Self {
        let status = match actix_web::http::StatusCode::from_u16(head.status) {
            Ok(status) => status,
            Err(_) => {
                warn!("invalid status code: {:?}", head.status);
                actix_web::http::StatusCode::INTERNAL_SERVER_ERROR
            }
        };

        let mut builder = actix_web::HttpResponseBuilder::new(status);
        for (key, value) in head.headers {
            if let Ok(header_value) = actix_web::http::header::HeaderValue::from_str(&value) {
                let _ = builder.append_header((key, header_value));
            }
        }
        builder
    }
}

impl Default for HTTPHead {
    fn default() -> Self {
        Self {
            status: 200,
            headers: vec![],
        }
    }
}

pub struct State {
    ctx: WasiCtx,
    table: ResourceTable,
    head: Arc<Mutex<HTTPHead>>,
}

impl State {
    pub fn new(stdin: InputStream, stdout: OutputStream, head: Arc<Mutex<HTTPHead>>) -> Self {
        let ctx = WasiCtxBuilder::new()
            .env("FUNCGG", "1")
            .stdin(AsyncStdinStream::from(stdin))
            .stdout(stdout)
            .inherit_stderr() // TODO(robherley): pipe stderr to a log stream
            .build();
        Self {
            ctx,
            table: ResourceTable::default(),
            head,
        }
    }
}

impl WasiView for State {
    fn ctx(&mut self) -> &mut WasiCtx {
        &mut self.ctx
    }
    fn table(&mut self) -> &mut ResourceTable {
        &mut self.table
    }
}

impl wit::funcgg::function::responder::Host for State {
    async fn set_status(&mut self, status: u16) {
        info!("set_status: {:?}", status);
        if let Ok(mut head) = self.head.lock() {
            head.status = status;
        }
    }

    async fn set_header(&mut self, key: String, value: String) {
        info!("set_header: {:?}={:?}", key, value);
        if let Ok(mut head) = self.head.lock() {
            head.headers.push((key, value));
        }
    }
}

pub struct Sandbox {
    engine: Engine,
    linker: Linker<State>,
    component: Component,
}

impl Sandbox {
    pub fn new(binary: Vec<u8>) -> Result<Self> {
        let start = std::time::Instant::now();
        let mut config = wasmtime::Config::default();
        config.debug_info(false);
        config.async_support(true);
        config.epoch_interruption(true);

        let engine = Engine::new(&config)?;
        let mut linker = Linker::new(&engine);
        wasmtime_wasi::add_to_linker_async(&mut linker)?;
        wit::Run::add_to_linker(&mut linker, |state: &mut State| state)?;

        let component = Component::new(&engine, binary)?;

        // TODO: cache serialized component
        info!("wasmtime init took: {:?}", start.elapsed());
        Ok(Self {
            engine,
            linker,
            component,
        })
    }

    pub async fn call(
        &mut self,
        stdin: InputStream,
        stdout: OutputStream,
        head: Arc<Mutex<HTTPHead>>,
    ) -> Result<(), anyhow::Error> {
        let state = State::new(stdin, stdout, head);
        let mut store = Store::new(&self.engine, state);
        store.set_epoch_deadline(1);

        let (finished_tx, finished_rx) = tokio::sync::oneshot::channel::<()>();
        spawn({
            let weak_engine = self.engine.weak();
            async move {
                tokio::select! {
                  _ = tokio::time::sleep(MAX_RUNTIME_DURATION) => {
                    warn!("cancelling request");
                    if let Some(engine) = weak_engine.upgrade() {
                      engine.increment_epoch();
                    }
                  }
                  _ = finished_rx => { /* do nothing */ }
                }
            }
        });

        let command = Command::instantiate_async(&mut store, &self.component, &self.linker).await?;
        // exit codes are "unstable" still: https://github.com/WebAssembly/wasi-cli/blob/d4fddec89fb9354509dbfa29a5557c58983f327a/wit/exit.wit#L15
        let result = command.wasi_cli_run().call_run(&mut store).await;
        _ = finished_tx.send(());

        match result {
            Ok(_) => Ok(()),
            Err(_) => Err(anyhow::anyhow!("wasi command failed")),
        }
    }
}
