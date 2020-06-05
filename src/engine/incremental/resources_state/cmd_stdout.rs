use crate::config::yaml;
use crate::run_script;
use anyhow::{anyhow, Context, Result};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

#[derive(Serialize, Deserialize, PartialEq)]
pub struct ResourcesState(HashMap<String, String>);

impl ResourcesState {
    pub fn current(resources: &[yaml::Resource], dir: &Path) -> Result<Self> {
        let state = get_cmds(resources)
            .par_iter()
            .map(|cmd| get_cmd_stdout(cmd, dir).map(|stdout| (cmd.to_string(), stdout)))
            .collect::<Result<_>>()?;

        Ok(Self(state))
    }

    pub fn eq_current_state(&self, resources: &[yaml::Resource], dir: &Path) -> bool {
        get_cmds(resources)
            .par_iter()
            .all(|cmd| match get_cmd_stdout(cmd, dir) {
                Ok(stdout) => self.0.get(&cmd.to_string()) == Some(&stdout),
                Err(e) => {
                    log::error!("Command {} failed to execute: {}", cmd, e);
                    false
                }
            })
    }
}

fn get_cmds(resources: &[yaml::Resource]) -> Vec<&str> {
    resources
        .iter()
        .filter_map(|resource| {
            if let yaml::Resource::CmdStdout { cmd_stdout } = resource {
                Some(cmd_stdout.as_ref())
            } else {
                None
            }
        })
        .collect()
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
