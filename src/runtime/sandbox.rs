use anyhow::{Result, anyhow};
use deno_core::url::Url;
use deno_core::{JsRuntime, RuntimeOptions};
use rustls::crypto::{CryptoProvider, aws_lc_rs};
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::LazyLock;
use std::time::Duration;
use tokio::time::timeout;
use uuid::Uuid;

use super::ext;
use super::http;
use super::loader;

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
        _ = CryptoProvider::install_default(aws_lc_rs::default_provider());

        let state = Rc::new(RefCell::new(State {
            request_id,
            ..Default::default()
        }));

        let extension_transpiler = Rc::new(loader::transpile);

        // TODO: snapshotting???
        let runtime = JsRuntime::try_new(RuntimeOptions {
            extensions: ext::extensions(),
            extension_transpiler: Some(extension_transpiler),
            ..Default::default()
        })?;

        runtime.op_state().borrow_mut().put(state.clone());

        Ok(Self { runtime, state })
    }

    pub async fn execute(
        &mut self,
        user_code: String,
        request: http::Request,
        timeout_duration: Duration,
    ) -> Result<http::Response> {
        let execution_result = timeout(timeout_duration, async {
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
            Ok::<(), anyhow::Error>(())
        })
        .await;

        match execution_result {
            Ok(Ok(())) => {
                // completed
            }
            Ok(Err(e)) => {
                // some error?
                return Err(e);
            }
            Err(_) => {
                // timeout
                return Err(anyhow!("JavaScript execution timed out"));
            }
        }

        let mut res: http::Response = self.state.borrow_mut().res.take().unwrap_or_default();
        res.default_and_validate()?;
        res.set_runtime_headers(self.state.borrow().request_id);

        Ok(res)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::time::Duration;

    #[tokio::test]
    async fn test_execute_timeout() {
        let request_id = Uuid::new_v4();
        let mut sandbox = Sandbox::new(request_id).expect("Failed to create sandbox");

        let long_running_code = r#"
            export default async function handler(request) {
                await new Promise(resolve => setTimeout(resolve, 1000));
                return { status: 200, headers: {}, body: "This should timeout" };
            }
        "#
        .to_string();

        let request = http::Request {
            method: "GET".to_string(),
            uri: "/".to_string(),
            headers: HashMap::new(),
            body: None,
        };

        let timeout_duration = Duration::from_millis(100);

        let result = sandbox
            .execute(long_running_code, request, timeout_duration)
            .await;

        assert!(result.is_err());
        let error_msg = result.unwrap_err().to_string();

        assert!(error_msg.contains("JavaScript execution timed out"));
    }

    #[tokio::test]
    async fn test_execute_success_within_timeout() {
        let request_id = Uuid::new_v4();
        let mut sandbox = Sandbox::new(request_id).expect("Failed to create sandbox");

        let simple_code = r#"
            export default function handler(request) {
                return { status: 200, headers: {}, body: "Hello, World!" };
            }
        "#
        .to_string();

        let request = http::Request {
            method: "GET".to_string(),
            uri: "/".to_string(),
            headers: HashMap::new(),
            body: None,
        };

        let timeout_duration = Duration::from_secs(5);

        let result = sandbox
            .execute(simple_code, request, timeout_duration)
            .await;

        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.status, 200);
        assert_eq!(response.body, "Hello, World!");
    }
}
