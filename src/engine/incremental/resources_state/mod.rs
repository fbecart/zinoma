mod cmd_stdout;
mod fs;

use crate::config::yaml;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Serialize, Deserialize, PartialEq)]
pub struct ResourcesState {
    fs: fs::ResourcesState,
    cmd_stdout: cmd_stdout::ResourcesState,
}

impl ResourcesState {
    pub fn current(resources: &[yaml::Resource], project_dir: &Path) -> Result<Self> {
        Ok(Self {
            fs: fs::ResourcesState::current(resources, project_dir)?,
            cmd_stdout: cmd_stdout::ResourcesState::current(resources, project_dir)?,
        })
    }

    pub fn eq_current_state(
        &self,
        resources: &[yaml::Resource],
        project_dir: &Path,
    ) -> Result<bool> {
        // TODO Parallelize this computation
        Ok((&self.fs).eq_current_state(resources, project_dir)?
            && (&self.cmd_stdout).eq_current_state(resources, project_dir))
    }
}
