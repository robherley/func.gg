use deno_console::deno_console;
use deno_core::{OpState, op2};
use deno_fetch::deno_fetch;
use deno_net::deno_net;
use deno_telemetry::deno_telemetry;
use deno_url::deno_url;
use deno_web::deno_web;
use deno_webidl::deno_webidl;
use std::cell::RefCell;
use std::rc::Rc;

use super::http;
use super::sandbox::State;

mod permissions;
use permissions::Permissions;

deno_core::extension!(
    funcgg_runtime,
    ops = [op_get_request, op_set_response, op_get_request_id, op_tls_peer_certificate],
    esm_entry_point = "ext:funcgg_runtime/funcgg_entrypoint.js",
    esm = [
        dir "src/runtime/ext",
        "funcgg_tmp.js",
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
        deno_web::init::<Permissions>(Default::default(), None),
        deno_net::init::<Permissions>(None, None),
        deno_fetch::init::<Permissions>(Default::default()),
        funcgg_runtime::init(),
    ]
}

fn get(op_state: &OpState) -> std::cell::Ref<'_, State> {
    let st = op_state.borrow::<Rc<RefCell<State>>>();
    st.borrow()
}

fn get_mut(op_state: &mut OpState) -> std::cell::RefMut<'_, State> {
    let st = op_state.borrow_mut::<Rc<RefCell<State>>>();
    st.borrow_mut()
}

#[op2]
#[serde]
fn op_get_request(state: &mut OpState) -> Option<http::Request> {
    get(state).req.clone()
}

#[op2]
fn op_set_response(state: &mut OpState, #[serde] res: http::Response) {
    get_mut(state).res = Some(res);
}

#[op2]
#[string]
fn op_get_request_id(state: &mut OpState) -> String {
    get(state).request_id.to_string()
}

#[deno_core::op2]
#[serde]
pub fn op_tls_peer_certificate(#[smi] _: u32, _: bool) -> Option<deno_core::serde_json::Value> {
    // For now, we won't support TLS peer certificate, so we return None.
    // This does not affect normal root certificate validation.
    // The "real" implementation would require pulling in the deno ext for node.
    // Unfortunately, this is required part of the tls implementation used by fetch.
    // https://github.com/denoland/deno/blob/daa412b0f2898a1c1e2184a6cb72b69f5806d6a5/ext/net/02_tls.js#L47-L49
    None
}
