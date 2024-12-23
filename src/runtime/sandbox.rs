use wasmtime::*;
use wasmtime_wasi::preview1::{self, WasiP1Ctx};
use wasmtime_wasi::{AsyncStdinStream, WasiCtxBuilder};

use crate::streams::{ReceiverStream, SenderStream};

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("wasmtime: {0}")]
    WasmtimeError(#[from] wasmtime::Error),
    #[error("non-zero exit code: {0}")]
    ExitCode(i32),
    #[error("other: {0}")]
    Other(String),
}

// TODO(robherley): adjust config for sandboxing
// ResourceLimiter: https://docs.rs/wasmtime/latest/wasmtime/trait.ResourceLimiter.html
// limiter: https://docs.rs/wasmtime/latest/wasmtime/struct.Store.html#method.limiter
// epoch_interruption: https://docs.rs/wasmtime/latest/wasmtime/struct.Config.html#method.epoch_interruption
// fuel: https://docs.rs/wasmtime/latest/wasmtime/struct.Config.html#method.consume_fuel

pub struct Sandbox {
    #[allow(dead_code)]
    config: Config,
    #[allow(dead_code)]
    engine: Engine,
    linker: Linker<WasiP1Ctx>,
    module: Module,
}

impl Sandbox {
    pub fn new(binary: Vec<u8>) -> Result<Self, Error> {
        let mut config = wasmtime::Config::default();
        config.debug_info(true);
        config.async_support(true);

        let engine = Engine::new(&config)?;

        let mut linker: Linker<WasiP1Ctx> = Linker::new(&engine);
        preview1::add_to_linker_async(&mut linker, |t| t)?;

        let module = Module::new(&engine, binary)?;

        Ok(Self {
            config,
            engine,
            linker,
            module,
        })
    }

    pub async fn handler(
        &mut self,
        stdin: ReceiverStream,
        stdout: SenderStream,
    ) -> Result<(), Error> {
        let wasi_ctx = WasiCtxBuilder::new()
            .env("FUNC_GG", "1")
            .stdin(AsyncStdinStream::from(stdin))
            .stdout(stdout)
            .inherit_stderr() // TODO(robherley): pipe stderr to a log stream
            .build_p1();

        let mut store = Store::new(&self.engine, wasi_ctx);

        let func = self
            .linker
            .module_async(&mut store, "", &self.module)
            .await?
            .get_default(&mut store, "")?
            .typed::<(), ()>(&store)?;

        let result = func.call_async(&mut store, ()).await.or_else(|err| {
            match err.downcast_ref::<wasmtime_wasi::I32Exit>() {
                Some(e) => {
                    if e.0 != 0 {
                        Err(Error::ExitCode(e.0))
                    } else {
                        Ok(())
                    }
                }
                _ => Err(err.into()),
            }
        })?;

        Ok(result)
    }
}
