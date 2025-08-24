use crate::runtime::*;
use deno_core::{op2, OpState};
use std::cell::RefCell;
use std::rc::Rc;

deno_core::extension!(
    funcgg_runtime,
    ops = [op_get_request, op_set_response, op_get_request_id],
    esm_entry_point = "ext:funcgg_runtime/99_entrypoint.js",
    esm = [dir "src/runtime/js/ext", "01_tmp.js", "99_entrypoint.js"]
);

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

#[op2]
#[string]
fn op_get_request_id(state: &mut OpState) -> String {
    let runtime_state = state.borrow::<Rc<RefCell<RuntimeState>>>();
    runtime_state.borrow().request_id.to_string()
}
