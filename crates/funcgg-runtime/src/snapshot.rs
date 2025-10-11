use deno_core::snapshot::CreateSnapshotOutput;
use std::env;
use std::rc::Rc;

use super::{ext, loader};

pub const FILE_NAME: &str = "FUNCGG_RUNTIME_SNAPSHOT.bin";

pub fn build() -> anyhow::Result<CreateSnapshotOutput> {
    let extension_transpiler = Rc::new(loader::transpile);
    let snapshot = deno_core::snapshot::create_snapshot(
        deno_core::snapshot::CreateSnapshotOptions {
            cargo_manifest_dir: env!("CARGO_MANIFEST_DIR"),
            startup_snapshot: None,
            skip_op_registration: false,
            extensions: ext::extensions(),
            extension_transpiler: Some(extension_transpiler),
            with_runtime_cb: None,
        },
        None,
    )?;

    Ok(snapshot)
}
