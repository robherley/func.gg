use deno_console::deno_console;
use deno_core::{JsBuffer, OpState, op2};
use deno_fetch::deno_fetch;
use deno_net::deno_net;
use deno_telemetry::deno_telemetry;
use deno_url::deno_url;
use deno_web::deno_web;
use deno_webidl::deno_webidl;
use std::cell::RefCell;
use std::rc::Rc;
use std::vec;

use super::http;
use super::sandbox::State;

mod permissions;
use permissions::Permissions;

deno_core::extension!(
    funcgg_runtime,
    ops = [
        op_get_request,
        op_set_response,
        op_get_request_id,
        op_tls_peer_certificate,
        op_read_request_chunk,
        op_write_response_chunk,
    ],
    esm_entry_point = "ext:funcgg_runtime/funcgg_entrypoint.js",
    esm = [
        dir "src/ext",
        "deno_webidl.js",
        "deno_console.js",
        "deno_url.js",
        "deno_web.js",
        "deno_net.js",
        "deno_fetch.js",
        "funcgg_entrypoint.js",
    ],
    state = |state| state.put(Permissions{}),
);

pub fn extensions() -> Vec<deno_core::Extension> {
    vec![
        deno_telemetry::init(),
        deno_webidl::init(),
        deno_console::init(),
        deno_url::init(),
        // TODO: look more into these configurations
        deno_web::init::<Permissions>(Default::default(), None),
        deno_net::init::<Permissions>(None, None),
        deno_fetch::init::<Permissions>(Default::default()),
        funcgg_runtime::init(),
    ]
}

#[derive(Debug, thiserror::Error, deno_error::JsError)]
pub enum JsError {
    #[class(type)]
    #[error("an internal error occurred: {0}")]
    Internal(String),
}

#[op2]
#[serde]
pub fn op_tls_peer_certificate(#[smi] _: u32, _: bool) -> Option<deno_core::serde_json::Value> {
    // For now, we won't support TLS peer certificate, so we return None.
    // This does not affect normal root certificate validation.
    // The "real" implementation would require pulling in the deno ext for node.
    // Unfortunately, this is required part of the tls implementation used by fetch.
    // https://github.com/denoland/deno/blob/daa412b0f2898a1c1e2184a6cb72b69f5806d6a5/ext/net/02_tls.js#L47-L49
    None
}

#[op2]
#[serde]
fn op_get_request(state: &mut OpState) -> Option<http::Request> {
    state.borrow::<Rc<RefCell<State>>>().borrow().req.clone()
}

#[op2(async)]
async fn op_set_response(
    state: Rc<RefCell<OpState>>,
    #[serde] mut res: http::Response,
) -> Result<(), JsError> {
    let (sender, request_id) = {
        let state_borrow = state.borrow();
        let sandbox_state = state_borrow.borrow::<Rc<RefCell<super::sandbox::State>>>();
        let mut borrowed = sandbox_state.borrow_mut();
        (borrowed.response_oneshot_tx.take(), borrowed.request_id)
    };

    res.apply_runtime_headers(request_id);
    res.default_and_validate()
        .map_err(|err| JsError::Internal(err.to_string()))?;

    let sender = match sender {
        Some(sender) => sender,
        None => return Err(JsError::Internal("Response already sent".to_string())),
    };

    if sender.send(res).is_err() {
        return Err(JsError::Internal("Unable to send response".to_string()));
    }

    Ok(())
}

#[op2]
#[string]
fn op_get_request_id(state: &mut OpState) -> String {
    state
        .borrow::<Rc<RefCell<State>>>()
        .borrow()
        .request_id
        .to_string()
}

#[op2(async)]
#[buffer]
async fn op_read_request_chunk(state: Rc<RefCell<OpState>>) -> Result<Vec<u8>, JsError> {
    let receiver = {
        let state_borrow = state.borrow();
        let sandbox_state = state_borrow.borrow::<Rc<RefCell<super::sandbox::State>>>();
        sandbox_state.borrow().incoming_body_rx.clone()
    };

    let chunk = receiver.lock().await.recv().await;

    match chunk {
        Some(Ok(chunk)) => Ok(chunk.into()),
        Some(Err(err)) => Err(JsError::Internal(err)),
        None => Ok(vec![]),
    }
}

#[op2(async)]
async fn op_write_response_chunk(
    state: Rc<RefCell<OpState>>,
    #[buffer] data: JsBuffer,
) -> Result<(), JsError> {
    let sender = {
        let state_borrow = state.borrow();
        let sandbox_state = state_borrow.borrow::<Rc<RefCell<super::sandbox::State>>>();
        sandbox_state.borrow().outgoing_body_tx.clone()
    };

    sender
        .send(bytes::Bytes::from(data.to_vec())) // TODO: BAD! this is a copy
        .await
        .map_err(|err| JsError::Internal(err.to_string()))?;

    Ok(())
}
