use anyhow::Result;
use deno_core::{op2, JsRuntime, OpState, RuntimeOptions};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::rc::Rc;
use std::cell::RefCell;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct HttpRequest {
    pub method: String,
    pub url: String,
    pub headers: HashMap<String, String>,
    pub body: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct HttpResponse {
    pub status: u16,
    pub headers: HashMap<String, String>,
    pub body: String,
}

#[derive(Default, Serialize, Deserialize)]
struct RuntimeState {
    req: Option<HttpRequest>,
    res: Option<HttpResponse>,
}

#[op2]
#[serde]
fn op_get_request(state: &mut OpState) -> Option<HttpRequest> {
    let runtime_state = state.borrow::<Rc<RefCell<RuntimeState>>>();
    runtime_state.borrow().req.clone()
}

#[op2]
fn op_set_response(state: &mut OpState, #[serde] res: HttpResponse) {
    let runtime_state = state.borrow_mut::<Rc<RefCell<RuntimeState>>>();
    runtime_state.borrow_mut().res = Some(res);
}

deno_core::extension!(
    funcgg_http,
    ops = [
        op_get_request,
        op_set_response
    ],
    esm_entry_point = "ext:funcgg_http/req-res.js",
    esm = ["ext:funcgg_http/req-res.js" = {
        source = r#"
        const { op_get_request, op_set_response } = Deno.core.ops;
        globalThis.Func = {
            request: {
                get: () => op_get_request(),
            },
            response: {
                set: (res) => op_set_response(res),
            },
        };
        "#
    }]
);

pub struct JavaScriptRuntime {
    runtime: JsRuntime,
    state: Rc<RefCell<RuntimeState>>,
}

impl JavaScriptRuntime {
    pub fn new() -> Result<Self> {
        let state = Rc::new(RefCell::new(RuntimeState::default()));
        let state_for_extension = state.clone();
        
        // TODO: snapshotting???
        let runtime = JsRuntime::new(RuntimeOptions {
            extensions: vec![
                funcgg_http::init(),
                // TODO: think about other extensions (like other stdlibs, kv, etc)
            ],
            ..Default::default()
        });

        runtime.op_state().borrow_mut().put(state_for_extension);

        Ok(Self { runtime, state })
    }

    pub async fn invoke_handler(&mut self, js_code: String, request: HttpRequest) -> Result<HttpResponse> {
        self.state.borrow_mut().req = Some(request);

        let mod_specifier = "func:user".parse().expect("bad module specifier");
        let mod_id = self.runtime
            .load_main_es_module_from_code(&mod_specifier, js_code)
            .await?;

        let result = self.runtime.mod_evaluate(mod_id);
        self.runtime.run_event_loop(Default::default()).await?;
        result.await?;

        let res = self.state.borrow_mut().res.take().ok_or_else(|| {
            anyhow::anyhow!("No response set in the runtime state")
        })?;

        // TODO: status validation, append/overwrite headers, etc

        Ok(res)
    }
}