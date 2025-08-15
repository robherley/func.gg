use anyhow::Result;
use deno_core::{op2, JsRuntime, OpState, RuntimeOptions, StaticModuleLoader};
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

        let module_loader = StaticModuleLoader::with(
            "func:http".parse().expect("bad module specifier"), // TODO: build full map of modules
            include_str!("./runtime/handler.js"),
        );
        
        // TODO: snapshotting???
        let runtime = JsRuntime::new(RuntimeOptions {
            extensions: vec![
                funcgg_http::init(),
                // TODO: think about other extensions (like other stdlibs, kv, etc)
            ],
            module_loader: Some(Rc::new(module_loader)),
            ..Default::default()
        });

        runtime.op_state().borrow_mut().put(state_for_extension);

        Ok(Self { runtime, state })
    }

    pub async fn invoke_handler(&mut self, request: HttpRequest) -> Result<HttpResponse> {
        
        // // This wrapper calls the user's handler and returns a Promise
        // let js_code = include_str!("./runtime/handler.js");

        // // Execute the wrapper script; the last expression evaluates to a Promise
        // let result = self.runtime.execute_script("<invoke>", js_code)?;

        // info!("finished executing handler");

        // {
        //     let scope = &mut self.runtime.handle_scope();
        //     let local_value = deno_core::v8::Local::new(scope, result.clone());
        //     let js_string = local_value.to_rust_string_lossy(scope);
        //     info!("raw JS result before resolving: {}", js_string);
        // }
        
        // // Await the Promise and drive the event loop until it settles
        // let response_value = self.runtime.resolve(result).await?;

        // info!("finished resolving");
        
        // // Convert the resolved V8 value to our Rust HttpResponse
        // let scope = &mut self.runtime.handle_scope();

        // info!("handle scope");
        
        // let local_value = deno_core::v8::Local::new(scope, response_value);

        // info!("local value");
        
        // let response: HttpResponse = deno_core::serde_v8::from_v8(scope, local_value)
        //     .map_err(|e| anyhow::anyhow!("Failed to deserialize response: {}", e))?;
    
        
        let main_mod_specifier = "func:handler".parse().expect("bad module specifier");
        let mod_id = self.runtime.
            load_main_es_module_from_code(
                &main_mod_specifier,
                include_str!("./runtime/handler.js"),
            ).await?;

        let result = self.runtime.mod_evaluate(mod_id);
        self.runtime.run_event_loop(Default::default()).await?;
        result.await?;

        Ok(HttpResponse { status: 200, headers: HashMap::new(), body: String::new() })
    }
}