use std::env;
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo:rerun-if-changed=build.rs");
    let out_dir = PathBuf::from(env::var("OUT_DIR")?);
    let snapshot = funcgg_runtime::snapshot::build()?;

    println!("cargo:rerun-if-changed=../funcgg-runtime/src");
    for file in snapshot.files_loaded_during_snapshot {
        println!("cargo:rerun-if-changed={}", file.display());
    }

    std::fs::write(out_dir.join("FUNCGG_RUNTIME_SNAPSHOT.bin"), snapshot.output)?;
    Ok(())
}
