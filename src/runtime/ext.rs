use deno_core::{op2, OpState};
use std::cell::RefCell;
use std::rc::Rc;

use super::http;
use super::sandbox::State;

deno_core::extension!(
    funcgg_runtime,
    ops = [op_get_request, op_set_response, op_get_request_id],
    esm_entry_point = "ext:funcgg_runtime/99_entrypoint.js",
    esm = [dir "src/runtime/js/ext", "01_tmp.js", "99_entrypoint.js"]
);

fn get(op_state: &OpState) -> std::cell::Ref<State> {
    let st = op_state.borrow::<Rc<RefCell<State>>>();
    st.borrow()
}

fn get_mut(op_state: &mut OpState) -> std::cell::RefMut<State> {
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
