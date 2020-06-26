use crate::run_script;
use anyhow::{anyhow, Context, Result};
use async_std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize, PartialEq)]
pub struct ResourcesState(HashMap<String, String>);

impl ResourcesState {
    pub fn current(cmds: &[(String, PathBuf)]) -> Result<Self> {
        // TODO Here was rayon
        let state = cmds
            .iter()
            .map(|(cmd, dir)| get_cmd_stdout(cmd, dir).map(|stdout| (cmd.to_string(), stdout)))
            .collect::<Result<_>>()?;

        Ok(Self(state))
    }

    pub fn eq_current_state(&self, cmds: &[(String, PathBuf)]) -> bool {
        // TODO Here was rayon
        cmds.iter()
            .all(|(cmd, dir)| match get_cmd_stdout(cmd, dir) {
                Ok(stdout) => self.0.get(&cmd.to_string()) == Some(&stdout),
                Err(e) => {
                    log::error!("Command {} failed to execute: {}", cmd, e);
                    false
                }
            })
    }
}

fn get_cmd_stdout(cmd: &str, dir: &Path) -> Result<String> {
    let output = run_script::build_command(cmd, dir)
        .output()
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
