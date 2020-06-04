use anyhow::{anyhow, Context, Result};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::{path::Path, process::Command};

#[derive(Serialize, Deserialize, PartialEq)]
pub struct ResourcesState(HashMap<String, String>);

impl ResourcesState {
    pub fn current(cmds: &[String], dir: &Path) -> Result<Self> {
        let state = cmds
            .par_iter()
            .map(|cmd| get_cmd_stdout(cmd, dir).map(|stdout| (cmd.to_owned(), stdout)))
            .collect::<Result<_>>()?;

        Ok(Self(state))
    }

    pub fn eq_current_state(&self, cmds: &[String], dir: &Path) -> bool {
        cmds.par_iter()
            .all(|cmd| match dbg!(get_cmd_stdout(cmd, dir)) {
                Ok(stdout) => dbg!(self.0.get(cmd)) == Some(&stdout),
                Err(e) => {
                    log::error!("Command {} failed to execute: {}", cmd, e);
                    false
                }
            })
    }
}

fn get_cmd_stdout(cmd: &str, dir: &Path) -> Result<String> {
    let (program, run_arg) = if cfg!(windows) {
        let comspec = std::env::var_os("COMSPEC").unwrap_or_else(|| "cmd.exe".into());
        (comspec, "/C")
    } else {
        ("/bin/sh".into(), "-c")
    };

    let output = Command::new(program)
        .arg(run_arg)
        .arg(cmd)
        .current_dir(dir)
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
