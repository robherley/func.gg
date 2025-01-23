use anyhow::Result;
use log::{info, warn};
use std::env;
use tokio::spawn;
use wasmtime::*;
use wasmtime_wasi::preview1::{self, WasiP1Ctx};
use wasmtime_wasi::{AsyncStdinStream, WasiCtxBuilder};

use crate::streams::{InputStream, OutputStream};

const MAX_RUNTIME_DURATION: std::time::Duration = std::time::Duration::from_secs(10);

// TODO(robherley): adjust config for sandboxing
// ResourceLimiter: https://docs.rs/wasmtime/latest/wasmtime/trait.ResourceLimiter.html
// limiter: https://docs.rs/wasmtime/latest/wasmtime/struct.Store.html#method.limiter
// epoch_interruption: https://docs.rs/wasmtime/latest/wasmtime/struct.Config.html#method.epoch_interruption
// fuel: https://docs.rs/wasmtime/latest/wasmtime/struct.Config.html#method.consume_fuel

pub struct Sandbox {
    #[allow(dead_code)]
    config: Config,
    engine: Engine,
    linker: Linker<WasiP1Ctx>,
    module: Module,
}

impl Sandbox {
    pub fn new(binary: Vec<u8>) -> Result<Self> {
        // NOTE: if config changes, we need to recompile the module
        let mut config = wasmtime::Config::default();
        config.debug_info(false);
        config.async_support(true);
        config.epoch_interruption(true);

        let engine = Engine::new(&config)?;

        let mut linker: Linker<WasiP1Ctx> = Linker::new(&engine);
        preview1::add_to_linker_async(&mut linker, |t| t)?;

        let cache_path =
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tmp/compile-cache");
        let module = if cache_path.exists() {
            info!("loading cached module from disk");
            unsafe { Module::deserialize_file(&engine, cache_path)? }
        } else {
            info!("compiling module from binary");
            let module = Module::new(&engine, binary)?;
            std::fs::write(cache_path, module.serialize()?)?;
            module
        };

        Ok(Self {
            config,
            engine,
            linker,
            module,
        })
    }

    pub async fn call(
        &mut self,
        stdin: InputStream,
        stdout: OutputStream,
    ) -> Result<i32, anyhow::Error> {
        let wasi_ctx = WasiCtxBuilder::new()
            .env("FUNCGG", "1")
            .stdin(AsyncStdinStream::from(stdin))
            .stdout(stdout)
            .inherit_stderr() // TODO(robherley): pipe stderr to a log stream
            .build_p1();

        // NOTE: if store changes, we need to recompile the module
        let mut store = Store::new(&self.engine, wasi_ctx);
        store.set_epoch_deadline(1);
        // TODO: limit memory with store.limiter();

        let func = self
            .linker
            .module_async(&mut store, "", &self.module)
            .await?
            .get_default(&mut store, "")?
            .typed::<(), ()>(&store)?;

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
