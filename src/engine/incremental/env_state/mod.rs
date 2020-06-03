mod cmd_stdout;
mod fs;

use crate::domain::EnvProbes;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Serialize, Deserialize, PartialEq)]
pub struct EnvState {
    fs: fs::EnvState,
    cmd_stdout: cmd_stdout::EnvState,
}

impl EnvState {
    pub fn current(env_probes: &EnvProbes, project_dir: &Path) -> Result<Self> {
        Ok(Self {
            fs: fs::EnvState::current(&env_probes.paths)?,
            cmd_stdout: cmd_stdout::EnvState::current(&env_probes.cmds, project_dir)?,
        })
    }

    pub fn eq_current_state(&self, env_probes: &EnvProbes, project_dir: &Path) -> Result<bool> {
        // TODO Parallelize this computation
        Ok((&self.fs).eq_current_state(&env_probes.paths)?
            && (&self.cmd_stdout).eq_current_state(&env_probes.cmds, project_dir))
    }
}
