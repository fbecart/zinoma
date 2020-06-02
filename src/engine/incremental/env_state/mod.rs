mod cmd_outputs;
mod fs;

use crate::domain::{EnvProbes, Target};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Serialize, Deserialize, PartialEq)]
pub struct TargetEnvState {
    inputs: EnvState,
    outputs: EnvState,
}

impl TargetEnvState {
    pub fn current(target: &Target) -> Result<Option<Self>> {
        if target.inputs.is_empty() {
            Ok(None)
        } else {
            let project_dir = &target.project.dir;
            Ok(Some(TargetEnvState {
                inputs: EnvState::current(&target.inputs, project_dir)?,
                outputs: EnvState::current(&target.outputs, project_dir)?,
            }))
        }
    }

    pub fn eq_current_state(&self, target: &Target) -> Result<bool> {
        let project_dir = &target.project.dir;
        Ok(self.inputs.eq_current_state(&target.inputs, &project_dir)?
            && self
                .outputs
                .eq_current_state(&target.outputs, &project_dir)?)
    }
}

#[derive(Serialize, Deserialize, PartialEq)]
struct EnvState {
    fs: fs::EnvState,
    cmd_stdouts: cmd_outputs::EnvState,
}

impl EnvState {
    fn current(env_probes: &EnvProbes, project_dir: &Path) -> Result<Self> {
        Ok(Self {
            fs: fs::EnvState::current(&env_probes.paths)?,
            cmd_stdouts: cmd_outputs::EnvState::current(&env_probes.cmd_outputs, project_dir)?,
        })
    }

    fn eq_current_state(&self, env_probes: &EnvProbes, project_dir: &Path) -> Result<bool> {
        Ok((&self.fs).eq_current_state(&env_probes.paths)?
            && (&self.cmd_stdouts).eq_current_state(&env_probes.cmd_outputs, project_dir)?)
    }
}

// TODO Run all computations in parallel?
