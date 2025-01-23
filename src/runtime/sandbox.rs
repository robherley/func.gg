use anyhow::Result;
use log::{info, warn};
use tokio::spawn;
use wasmtime::component::{Component, Linker};
use wasmtime::{Engine, Store};
use wasmtime_wasi::*;

use crate::streams::{InputStream, OutputStream};
use crate::wit;

const MAX_RUNTIME_DURATION: std::time::Duration = std::time::Duration::from_secs(10);

// TODO(robherley): adjust config for sandboxing
// ResourceLimiter: https://docs.rs/wasmtime/latest/wasmtime/trait.ResourceLimiter.html
// limiter: https://docs.rs/wasmtime/latest/wasmtime/struct.Store.html#method.limiter
// epoch_interruption: https://docs.rs/wasmtime/latest/wasmtime/struct.Config.html#method.epoch_interruption
// fuel: https://docs.rs/wasmtime/latest/wasmtime/struct.Config.html#method.consume_fuel

pub struct State {
    ctx: WasiCtx,
    table: ResourceTable,
}

impl State {
    pub fn new(stdin: InputStream, stdout: OutputStream) -> Self {
        let ctx = WasiCtxBuilder::new()
            .env("FUNCGG", "1")
            .stdin(AsyncStdinStream::from(stdin))
            .stdout(stdout)
            .inherit_stderr() // TODO(robherley): pipe stderr to a log stream
            .build();
        Self {
            ctx,
            table: ResourceTable::default(),
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

#[async_trait]
impl wit::funcgg::runtime::fetcher::Host for State {
    async fn fetch(&mut self, input: Vec<u8>) -> Vec<u8> {
        info!("fetching: {:?}", input);
        "hello world".as_bytes().to_vec()
    }
}

pub struct Sandbox {
    engine: Engine,
    linker: Linker<State>,
    component: Component,
}

impl Sandbox {
    pub fn new(binary: Vec<u8>) -> Result<Self> {
        // NOTE: if config changes, we need to recompile the module
        let mut config = wasmtime::Config::default();
        config.debug_info(false);
        config.async_support(true);
        config.epoch_interruption(true);

        let engine = Engine::new(&config)?;

        let mut linker = Linker::new(&engine);
        wasmtime_wasi::add_to_linker_async(&mut linker)?;
        wit::Runner::add_to_linker(&mut linker, |state: &mut State| state)?;

        let component = Component::new(&engine, binary)?;

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
    ) -> Result<i32, anyhow::Error> {
        let state = State::new(stdin, stdout);
        let mut store = Store::new(&self.engine, state);
        store.set_epoch_deadline(1);

        let instance = self
            .linker
            .instantiate_async(&mut store, &self.component)
            .await?;

        let func = instance.get_typed_func::<(), ()>(&mut store, "_start")?;
        let (finished_tx, finished_rx) = tokio::sync::oneshot::channel::<()>();

        spawn({
            let weak_engine = self.engine.weak();
            async move {
                tokio::select! {
                  _ = tokio::time::sleep(MAX_RUNTIME_DURATION) => {
                    warn!("cancelling request");
                    match weak_engine.upgrade() {
                      Some(engine) => {
                        engine.increment_epoch();
                      }
                      None => {
                        warn!("engine dropped before interrupting");
                      }
                    }
                  }
                  _ = finished_rx => { /* do nothing */ }
                }
            }
        });

        let result = func.call_async(&mut store, ()).await;
        _ = finished_tx.send(());

        let mut exit_code = -1;
        if let Err(err) = result {
            if let Some(exit) = err.downcast_ref::<wasmtime_wasi::I32Exit>() {
                exit_code = exit.0;
            } else {
                warn!("finished with error: {:?}", err);
                return Err(err.into());
            }
        }

        info!("exited with code: {:?}", exit_code);
        Ok(exit_code)
    }
}
