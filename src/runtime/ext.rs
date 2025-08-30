use ::deno_console::deno_console;
use ::deno_fetch::{FetchPermissions, deno_fetch};
use ::deno_net::{NetPermissions, deno_net};
use ::deno_telemetry::deno_telemetry;
use ::deno_url::deno_url;
use ::deno_web::{TimersPermission, deno_web};
use ::deno_webidl::deno_webidl;
use deno_core::url::Url;
use deno_core::{OpState, op2};
use deno_permissions::{CheckedPath, OpenAccessKind, PermissionCheckError, PermissionDeniedError};
use std::borrow::Cow;
use std::cell::RefCell;
use std::path::Path;
use std::rc::Rc;

use super::http;
use super::sandbox::State;

deno_core::extension!(
    funcgg_runtime,
    ops = [op_get_request, op_set_response, op_get_request_id, op_tls_peer_certificate],
    esm_entry_point = "ext:funcgg_runtime/funcgg_entrypoint.js",
    esm = [
        dir "src/runtime/js/ext",
        "funcgg_tmp.js",
        "init.deno_webidl.js",
        "init.deno_console.js",
        "init.deno_url.js",
        "init.deno_web.js",
        "init.deno_net.js",
        "init.deno_fetch.js",
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

struct Permissions {}

impl TimersPermission for Permissions {
    fn allow_hrtime(&mut self) -> bool {
        false
    }
}

impl FetchPermissions for Permissions {
    fn check_net_url(&mut self, _url: &Url, _api_name: &str) -> Result<(), PermissionCheckError> {
        Ok(())
    }

    fn check_open<'a>(
        &mut self,
        _path: Cow<'a, Path>,
        _open_access: OpenAccessKind,
        api_name: &str,
    ) -> Result<CheckedPath<'a>, PermissionCheckError> {
        Err(PermissionCheckError::PermissionDenied(
            PermissionDeniedError {
                access: api_name.to_string(),
                name: "open",
            },
        ))
    }

    fn check_net_vsock(
        &mut self,
        _cid: u32,
        _port: u32,
        api_name: &str,
    ) -> Result<(), PermissionCheckError> {
        Err(PermissionCheckError::PermissionDenied(
            PermissionDeniedError {
                access: api_name.to_string(),
                name: "net_vsock",
            },
        ))
    }
}

impl NetPermissions for Permissions {
    fn check_net<T: AsRef<str>>(
        &mut self,
        _host: &(T, Option<u16>),
        _api_name: &str,
    ) -> Result<(), PermissionCheckError> {
        Ok(())
    }

    fn check_open<'a>(
        &mut self,
        _path: Cow<'a, Path>,
        _open_access: OpenAccessKind,
        api_name: &str,
    ) -> Result<CheckedPath<'a>, PermissionCheckError> {
        Err(PermissionCheckError::PermissionDenied(
            PermissionDeniedError {
                access: api_name.to_string(),
                name: "open",
            },
        ))
    }

    fn check_vsock(
        &mut self,
        _cid: u32,
        _port: u32,
        api_name: &str,
    ) -> Result<(), PermissionCheckError> {
        Err(PermissionCheckError::PermissionDenied(
            PermissionDeniedError {
                access: api_name.to_string(),
                name: "net_vsock",
            },
        ))
    }
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
    None
}
