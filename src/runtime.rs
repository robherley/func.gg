use wasmtime::*;
use wasmtime_wasi::preview1::{self, WasiP1Ctx};
use wasmtime_wasi::{AsyncStdinStream, WasiCtxBuilder};

use crate::streams::{ReceiverStream, SenderStream};

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("wasmtime: {0}")]
    WasmtimeError(#[from] wasmtime::Error),
    #[error("wasi: {0}")]
    WasiError(#[from] wasi_common::Error),
    #[error("string array: {0}")]
    StringArrayError(#[from] wasi_common::StringArrayError),
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

pub async fn handler(
    binary: impl AsRef<[u8]>,
    stdin: ReceiverStream,
    stdout: SenderStream,
) -> Result<(), Error> {
    let mut cfg = wasmtime::Config::default();
    // cfg.debug_info(true);
    cfg.async_support(true);
    let engine = Engine::new(&cfg)?;
    let mut linker: Linker<WasiP1Ctx> = Linker::new(&engine);
    preview1::add_to_linker_async(&mut linker, |t| t)?;
    let module = Module::new(&engine, binary)?;

    let wasi_ctx = WasiCtxBuilder::new()
        .env("WEBFUNC", "1")
        .stdin(AsyncStdinStream::from(stdin))
        .stdout(stdout)
        .inherit_stderr() // TODO(robherley): pipe stderr to a log stream
        .build_p1();

    let mut store = Store::new(&engine, wasi_ctx);

    let func = linker
        .module_async(&mut store, "", &module)
        .await?
        .get_default(&mut store, "")?
        .typed::<(), ()>(&store)?;

    let result = func
        .call_async(&mut store, ())
        .await
        .or_else(|err| match err.downcast_ref::<wasmtime_wasi::I32Exit>() {
            Some(e) => {
                if e.0 != 0 {
                    Err(Error::ExitCode(e.0))
                } else {
                    Ok(())
                }
            }
            _ => Err(err.into()),
        })?;

    Ok(result)
}
