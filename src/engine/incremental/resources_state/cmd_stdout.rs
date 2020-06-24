use crate::run_script;
use anyhow::{anyhow, Context, Result};
use async_std::task;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Serialize, Deserialize, PartialEq)]
pub struct ResourcesState(HashMap<String, String>);

impl ResourcesState {
    pub fn current(cmds: &[(String, PathBuf)]) -> Result<Self> {
        let state = cmds
            .par_iter()
            .map(|(cmd, dir)| get_cmd_stdout(cmd, dir).map(|stdout| (cmd.to_string(), stdout)))
            .collect::<Result<_>>()?;

        Ok(Self(state))
    }

    pub fn eq_current_state(&self, cmds: &[(String, PathBuf)]) -> bool {
        cmds.par_iter()
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
    let output = task::block_on(async { run_script::build_command(cmd, dir).output().await })
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
