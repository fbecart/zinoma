mod cmd_outputs;
mod fs;

use crate::domain::EnvProbes;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Serialize, Deserialize, PartialEq)]
pub struct EnvState {
    fs: fs::EnvState,
    cmd_stdouts: cmd_outputs::EnvState,
}

impl EnvState {
    pub fn current(env_probes: &EnvProbes, project_dir: &Path) -> Result<Self> {
        Ok(Self {
            fs: fs::EnvState::current(&env_probes.paths)?,
            cmd_stdouts: cmd_outputs::EnvState::current(&env_probes.cmd_outputs, project_dir)?,
        })
    }

    pub fn eq_current_state(&self, env_probes: &EnvProbes, project_dir: &Path) -> Result<bool> {
        Ok((&self.fs).eq_current_state(&env_probes.paths)?
            && (&self.cmd_stdouts).eq_current_state(&env_probes.cmd_outputs, project_dir)?)
    }
}

// TODO Run all computations in parallel?
