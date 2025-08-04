use anyhow::Result;
use deno_core::{JsRuntime, RuntimeOptions, op2, OpState};
use log::info;
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
    current_request: Option<HttpRequest>,
}

#[op2]
#[serde]
fn op_get_request(state: &mut OpState) -> Option<HttpRequest> {
    let runtime_state = state.borrow::<Rc<RefCell<RuntimeState>>>();
    runtime_state.borrow().current_request.clone()
}

deno_core::extension!(
    funcgg_http,
    ops = [op_get_request],
    esm_entry_point = "ext:funcgg_http/runtime.js",
    esm = ["ext:funcgg_http/runtime.js" = {
        source = r#"
        const { op_get_request } = Deno.core.ops;
        globalThis.getRequest = function() {
            return op_get_request();
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

    pub async fn load_handler(&mut self, js_code: String) -> Result<()> {
        self.runtime.execute_script("<handler>", js_code)?;
        self.runtime.run_event_loop(Default::default()).await?;
        Ok(())
    }

    pub async fn invoke_handler(&mut self, request: HttpRequest) -> Result<HttpResponse> {
        self.state.borrow_mut().current_request = Some(request);
        
        let js_code = include_str!("./runtime/handler.js");

        // Execute the JavaScript and get the result
        let result = self.runtime.execute_script("<invoke>", js_code)?;

        info!("finished executing handler");

        {
            let scope = &mut self.runtime.handle_scope();
            let local_value = deno_core::v8::Local::new(scope, result.clone());
            let js_string = local_value.to_rust_string_lossy(scope);
            info!("raw JS result before resolving: {}", js_string);
        }
        
        // Resolve any promises
        let response_value = self.runtime.resolve(result).await?;

        info!("finished resolving");
        
        // Get a handle scope for V8 operations
        let scope = &mut self.runtime.handle_scope();

        info!("handle scope");
        
        // Convert the global handle to a local handle
        let local_value = deno_core::v8::Local::new(scope, response_value);

        info!("local value");
        
        // Directly deserialize the V8 value to our HttpResponse struct
        let response: HttpResponse = deno_core::serde_v8::from_v8(scope, local_value)
            .map_err(|e| anyhow::anyhow!("Failed to deserialize response: {}", e))?;

        info!("deserialized response: {:?}", response);
        
        // Clear the request data
        self.state.borrow_mut().current_request = None;
        
        Ok(response)
    }
}
