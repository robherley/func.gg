use anyhow::{anyhow, Result};
use deno_core::url::Url;
use deno_core::{JsRuntime, RuntimeOptions};
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::LazyLock;
use uuid::Uuid;

use super::ext;
use super::http;

static WORKER_CODE: &str = include_str!("./js/worker.js");
static WORKER_MOD_SPECIFIER: LazyLock<Url> =
    LazyLock::new(|| "func:worker".parse().expect("bad module specifier"));

static USER_MOD_SPECIFIER: LazyLock<Url> =
    LazyLock::new(|| "func:user-code".parse().expect("bad module specifier"));

#[derive(Default, Serialize, Deserialize)]
pub struct State {
    pub req: Option<http::Request>,
    pub res: Option<http::Response>,
    pub request_id: Uuid,
}

pub struct Sandbox {
    pub runtime: JsRuntime,
    pub state: Rc<RefCell<State>>,
}

impl Sandbox {
    pub fn new(request_id: Uuid) -> Result<Self> {
        let state = Rc::new(RefCell::new(State {
            request_id,
            ..Default::default()
        }));

        // TODO: snapshotting???
        let runtime = JsRuntime::new(RuntimeOptions {
            extensions: vec![
                ext::funcgg_runtime::init(),
                // TODO: think about other extensions (like other stdlibs, kv, etc)
            ],
            ..Default::default()
        });

        runtime.op_state().borrow_mut().put(state.clone());

        Ok(Self { runtime, state })
    }

    pub async fn execute(
        &mut self,
        user_code: String,
        request: http::Request,
    ) -> Result<http::Response> {
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

        let res = self
            .state
            .borrow_mut()
            .res
            .take()
            .ok_or_else(|| anyhow!("No response set in the runtime state"))?;

        // TODO: status validation, append/overwrite headers, etc

        Ok(res)
    }
}
