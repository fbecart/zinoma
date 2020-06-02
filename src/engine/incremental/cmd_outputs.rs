use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

#[derive(Serialize, Deserialize, PartialEq)]
pub struct EnvCmdOutputsState(HashMap<String, String>);

impl EnvCmdOutputsState {
    pub fn current(cmds: &[String], dir: &Path) -> Result<Self> {
        let state = cmds
            .iter()
            .map(|cmd| {
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

                Ok((cmd.to_owned(), output))
            })
            .collect::<Result<_>>()?;

        Ok(Self(state))
    }

    pub fn eq_current_state(&self, cmds: &[String], dir: &Path) -> Result<bool> {
        Ok(Self::current(cmds, dir)? == *self)
    }
}
