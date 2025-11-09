use anyhow::{Context, Result};
use std::env;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::process::Stdio;
use tokio::process::{Child, Command};
use tracing::{info, warn};

use crate::config::Config;

pub struct Paths {
    pub handler: PathBuf,
    pub script: PathBuf,
    pub socket: PathBuf,
    pub bun: Option<PathBuf>,
}

impl From<&Config> for Paths {
    fn from(config: &Config) -> Self {
        Self {
            handler: config.handler_path.clone(),
            script: config.script_path.clone(),
            socket: config.socket_path.clone(),
            bun: config.bun_path.clone(),
        }
    }
}

pub struct Process {
    paths: Paths,
    child: Option<Child>,
}

impl Process {
    pub fn new(paths: Paths) -> Self {
        Self { paths, child: None }
    }

    pub async fn spawn(&mut self) -> Result<()> {
        if self.child.is_some() {
            warn!("Runtime process is already running");
            return Ok(());
        }

        let bun_path = match self.paths.bun {
            Some(ref path) => path.clone(),
            None => which("bun").ok_or_else(|| anyhow::anyhow!("Unable to find executable"))?,
        };

        let mut command = Command::new(bun_path);
        command
            .env_clear()
            .env("FUNCD_SOCKET", &self.paths.socket)
            .env("FUNCD_SCRIPT", &self.paths.script)
            .arg("run")
            .arg(&self.paths.handler)
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
        if let Some(mut child) = self.child.take()
            && let Some(pid) = child.id()
        {
            info!(pid, "dropping runtime, killing process");
            let _ = child.start_kill();
        }
    }
}

fn which(bin: &str) -> Option<PathBuf> {
    let paths = env::var_os("PATH")?;
    for dir in env::split_paths(&paths) {
        let candidate = dir.join(bin);
        if candidate.is_file()
            && candidate
                .metadata()
                .map(|m| m.permissions().mode() & 0o111 != 0)
                .unwrap_or(false)
        {
            return Some(candidate);
        }
    }
    None
}
