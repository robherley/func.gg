use exports::wunc::handlers::http::{Method, Request, Response};
use wasmtime::component::{Component, Linker};
use wasmtime::{Engine, Store};
use wasmtime_wasi::*;

wasmtime::component::bindgen!({
  world: "handlers",
  path: "wit/wunc.wit",
  async: true,
});

pub struct State {
    pub data: Vec<u8>,
    ctx: WasiCtx,
    table: ResourceTable,
}

impl State {
    fn new() -> Self {
        let ctx = WasiCtxBuilder::new()
            .env("IS_WUNC", "TRUE")
            .stdout(stdout())
            .build();
        let table = ResourceTable::default();
        Self {
            ctx,
            table,
            data: vec![],
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
impl wunc::handlers::read_writer::Host for State {
    async fn read(&mut self, _n: i32) -> (Vec<u8>, i32) {
        let response = "rob";
        (response.as_bytes().to_vec(), response.len() as i32)
    }

    async fn write(&mut self, data: Vec<u8>) -> i32 {
        self.data.extend(data);
        self.data.len() as i32
    }
}

pub struct Runtime {
    engine: Engine,
    linker: Linker<State>,
    component: Component,
}

impl Runtime {
    pub fn new(binary: impl AsRef<[u8]>) -> wasmtime::Result<Self> {
        let mut cfg = wasmtime::Config::default();
        cfg.async_support(true);

        let engine = Engine::new(&cfg)?;
        let mut linker = Linker::new(&engine);
        wasmtime_wasi::add_to_linker_async(&mut linker)?;
        Handlers::add_to_linker(&mut linker, |state: &mut State| state)?;
        let component = Component::new(&engine, binary)?;

        Ok(Self {
            engine,
            linker,
            component,
        })
    }

    pub async fn handle(&mut self) -> wasmtime::Result<(Response, Store<State>)> {
        let mut store = Store::new(&self.engine, State::new());
        let bindings =
            Handlers::instantiate_async(&mut store, &self.component, &self.linker).await?;

        let response = bindings
            .wunc_handlers_http()
            .call_handle(
                &mut store,
                &Request {
                    method: Method::Get,
                    url: "https://example.com".to_string(),
                },
            )
            .await?;

        Ok((response, store))
    }
}
