use crate::async_utils;
use crate::{domain::CmdResource, run_script};
use anyhow::{anyhow, Context, Result};
use async_std::task;
use futures::future;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize, PartialEq)]
pub struct ResourcesState(HashMap<String, String>);

impl ResourcesState {
    pub async fn current(cmds: &[CmdResource]) -> Result<Self> {
        let futures = cmds.iter().map(|resource| async move {
            get_cmd_stdout(resource)
                .await
                .map(|stdout| (resource.cmd.to_string(), stdout))
        });

        let vec = future::try_join_all(futures).await?;
        Ok(Self(vec.into_iter().collect()))
    }

    pub async fn eq_current_state(&self, cmds: &[CmdResource]) -> bool {
        let futures = cmds.iter().map(|resource| async move {
            match get_cmd_stdout(resource).await {
                Ok(stdout) => self.0.get(&resource.cmd) == Some(&stdout),
                Err(e) => {
                    log::error!("Command {} failed to execute: {}", resource.cmd, e);
                    false
                }
            }
        });

        async_utils::all(futures).await
    }
}

async fn get_cmd_stdout(resource: &CmdResource) -> Result<String> {
    let mut command = run_script::build_command(&resource.cmd, &resource.dir);
    let output = task::spawn_blocking(move || command.output())
        .await
        .with_context(|| format!("Failed to run command {}", resource.cmd))?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(output.stdout.as_slice()).to_string())
    } else {
        Err(anyhow!(
            "Command {} returned {}",
            resource.cmd,
            output.status
        ))
    }
}
