mod cmd_stdout;
mod fs;

use crate::domain::Resources;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Serialize, Deserialize, PartialEq)]
pub struct ResourcesState {
    fs: fs::ResourcesState,
    cmd_stdout: cmd_stdout::ResourcesState,
}

impl ResourcesState {
    pub fn current(resources: &Resources, project_dir: &Path) -> Result<Self> {
        Ok(Self {
            fs: fs::ResourcesState::current(&resources.paths)?,
            cmd_stdout: cmd_stdout::ResourcesState::current(&resources.cmds, project_dir)?,
        })
    }

    pub fn eq_current_state(&self, resources: &Resources, project_dir: &Path) -> Result<bool> {
        // TODO Parallelize this computation
        Ok((&self.fs).eq_current_state(&resources.paths)?
            && (&self.cmd_stdout).eq_current_state(&resources.cmds, project_dir))
    }
}
