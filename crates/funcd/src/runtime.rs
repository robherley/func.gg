use anyhow::{Context, Result};
use std::env;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use tokio::process::{Child, Command};
use tracing::{info, warn};

const BUN_BINARY_NAME: &str = "bun";

pub struct Process {
    handler_path: PathBuf,
    socket_path: PathBuf,
    child: Option<Child>,
}

impl Process {
    pub fn new<P: AsRef<Path>>(handler_path: P, socket_path: P) -> Self {
        Self {
            handler_path: handler_path.as_ref().to_path_buf(),
            socket_path: socket_path.as_ref().to_path_buf(),
            child: None,
        }
    }

    pub async fn spawn(&mut self) -> Result<()> {
        if self.child.is_some() {
            warn!("Runtime process is already running");
            return Ok(());
        }

        let binary_path =
            which(BUN_BINARY_NAME).ok_or_else(|| anyhow::anyhow!("Unable to find executable"))?;

        let mut command = Command::new(binary_path);
        command
            .env_clear()
            .env("FUNCD_SOCKET", &self.socket_path)
            .arg("run")
            .arg(&self.handler_path)
            .stdin(Stdio::null())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .kill_on_drop(true);

        let child = command.spawn()?;

        info!(pid = child.id(), "runtime process spawned");
        self.child = Some(child);

        Ok(())
    }

    pub async fn wait(&mut self) -> Result<std::process::ExitStatus> {
        if let Some(mut child) = self.child.take() {
            child
                .wait()
                .await
                .context("failed to wait for runtime process")
        } else {
            anyhow::bail!("No runtime process to wait for")
        }
    }

    pub async fn _kill(&mut self) -> Result<()> {
        if let Some(mut child) = self.child.take() {
            info!(pid = child.id(), "killing runtime process");
            child
                .kill()
                .await
                .context("failed to kill runtime process")?;
            Ok(())
        } else {
            Ok(())
        }
    }
}

impl Drop for Process {
    fn drop(&mut self) {
        if let Some(mut child) = self.child.take() {
            if let Some(pid) = child.id() {
                info!(pid, "dropping runtime, killing process");
                let _ = child.start_kill();
            }
        }
    }
}

fn which(bin: &str) -> Option<String> {
    let paths = env::var_os("PATH")?;
    for dir in env::split_paths(&paths) {
        let candidate = dir.join(bin);
        if candidate.is_file()
            && candidate
                .metadata()
                .map(|m| m.permissions().mode() & 0o111 != 0)
                .unwrap_or(false)
        {
            return Some(candidate.display().to_string());
        }
    }
    None
}
