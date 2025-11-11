use anyhow::{Context, Result};
use std::process::Stdio;
use tokio::process::{Child, Command};
use tracing::{info, warn};

use crate::config::Paths;

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

        let mut command = Command::new(self.paths.bun.clone());
        command
            .env_clear()
            .env("FUNCD_MSG_SOCKET", &self.paths.msg_socket)
            .env("FUNCD_HTTP_SOCKET", &self.paths.http_socket)
            .env("FUNCD_SCRIPT", &self.paths.user_script)
            .arg("run")
            .arg(&self.paths.entry_point)
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
