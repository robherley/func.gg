use std::env;
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo:rerun-if-changed=build.rs");
    let out_dir = PathBuf::from(env::var("OUT_DIR")?);
    let snapshot = func_runtime::snapshot::build()?;

    println!("cargo:rerun-if-changed=../func_runtime/src");
    for file in snapshot.files_loaded_during_snapshot {
        println!("cargo:rerun-if-changed={}", file.display());
    }

    let snapshot_path = out_dir.join(func_runtime::snapshot::FILE_NAME);
    std::fs::write(snapshot_path.clone(), snapshot.output)?;
    println!("cargo:rustc-env=SNAPSHOT_PATH={}", snapshot_path.display());
    Ok(())
}
