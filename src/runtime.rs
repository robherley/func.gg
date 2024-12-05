use futures::StreamExt;
use wasmtime::component::{Linker, ResourceTable};
use wasmtime::{Engine, Module};
use wasmtime_wasi::{add_to_linker_sync, AsyncStdinStream, WasiCtx, WasiCtxBuilder, WasiView};

use crate::stream::ByteStream;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("wasmtime error: {0}")]
    WasmtimeError(#[from] wasmtime::Error),
}

pub struct State {
    ctx: WasiCtx,
    table: ResourceTable,
}

impl State {
    fn new(mut input: ByteStream) -> Self {
        let ctx = WasiCtxBuilder::new().env("WEBFUNC", "1").build();
        let table = ResourceTable::default();

        Self { ctx, table }
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

pub struct Sandbox {
    engine: Engine,
    linker: Linker<State>,
    module: Module,
}

impl Sandbox {
    pub fn new(binary: impl AsRef<[u8]>) -> Result<Self, Error> {
        let cfg = wasmtime::Config::default();
        let engine = Engine::new(&cfg)?;
        let mut linker = Linker::new(&engine);
        add_to_linker_sync(&mut linker)?;
        let module = Module::new(&engine, binary)?;
        // TODO: module cache to speed up execution

        Ok(Self {
            engine,
            linker,
            module,
        })
    }

    pub async fn handle(&mut self, mut stream: ByteStream) -> Result<(), Error> {
        println!("start handle");
        while let Some(item) = stream.next().await {
            println!("Chunk: {:?}", &item);
        }
        println!("end handle");

        Ok(())
    }
}
