use crate::run_script;
use anyhow::{anyhow, Context, Result};
use async_std::path::{Path, PathBuf};
use async_std::task;
use futures::future;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize, PartialEq)]
pub struct ResourcesState(HashMap<String, String>);

impl ResourcesState {
    pub async fn current(cmds: &[(String, PathBuf)]) -> Result<Self> {
        let futures = cmds.iter().map(|(cmd, dir)| async move {
            get_cmd_stdout(cmd, dir)
                .await
                .map(|stdout| (cmd.to_string(), stdout))
        });

        let vec = future::try_join_all(futures).await?;
        Ok(Self(vec.into_iter().collect()))
    }

    pub async fn eq_current_state(&self, cmds: &[(String, PathBuf)]) -> bool {
        // TODO Resolve at the first negative result
        let futures = cmds.iter().map(|(cmd, dir)| async move {
            match get_cmd_stdout(cmd, dir).await {
                Ok(stdout) => self.0.get(&cmd.to_string()) == Some(&stdout),
                Err(e) => {
                    log::error!("Command {} failed to execute: {}", cmd, e);
                    false
                }
            }
        });

        future::join_all(futures).await.into_iter().all(|r| r)
    }
}

async fn get_cmd_stdout(cmd: &str, dir: &Path) -> Result<String> {
    let mut command = run_script::build_command(cmd, dir);
    let output = task::spawn_blocking(move || command.output())
        .await
        .with_context(|| format!("Failed to run command {}", cmd))?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(output.stdout.as_slice()).to_string())
    } else {
        Err(anyhow!(
            "Command {} return error code {}",
            cmd,
            output.status
        ))
    }
}
