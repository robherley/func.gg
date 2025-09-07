use crate::runtime::loader;

use super::ext;
use std::env;
use std::path::PathBuf;
use std::rc::Rc;

pub const SNAPSHOT_FILE: &str = "FUNCGG_SNAPSHOT.bin";

pub struct Snapshotter {
    out_dir: PathBuf,
}

impl Snapshotter {
    pub fn new(out_dir: PathBuf) -> Self {
        Snapshotter { out_dir }
    }

    pub fn build(&self) -> anyhow::Result<()> {
        let snapshot_path = self.out_dir.join(SNAPSHOT_FILE);

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

        std::fs::write(snapshot_path, snapshot.output)?;

        Ok(())
    }
}
