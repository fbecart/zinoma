use anyhow::Result;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

#[derive(Serialize, Deserialize, PartialEq)]
pub struct EnvState(HashMap<String, String>);

impl EnvState {
    pub fn current(cmds: &[String], dir: &Path) -> Result<Self> {
        let state = cmds
            .par_iter()
            .map(|cmd| get_cmd_stdout(cmd, dir).map(|stdout| (cmd.to_owned(), stdout)))
            .collect::<Result<_>>()?;

        Ok(Self(state))
    }

    pub fn eq_current_state(&self, cmds: &[String], dir: &Path) -> bool {
        cmds.par_iter().all(|cmd| match get_cmd_stdout(cmd, dir) {
            Ok(stdout) => self.0.get(cmd) == Some(&stdout),
            Err(e) => {
                log::error!("Command {} failed to execute: {}", cmd, e);
                false
            }
        })
    }
}

fn get_cmd_stdout(cmd: &str, dir: &Path) -> Result<String> {
    let mut options = run_script::ScriptOptions::new();
    options.exit_on_error = true;
    options.working_directory = Some(dir.to_path_buf());

    let (code, output, _error) = run_script::run(cmd, &vec![], &options).unwrap();
    if code != 0 {
        return Err(anyhow::anyhow!(
            "Command {} return error code {}",
            cmd,
            code
        ));
    }

    Ok(output)
}
