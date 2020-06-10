mod cmd_stdout;
mod fs;

use crate::domain::Resources;
use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, PartialEq)]
pub struct ResourcesState {
    fs: fs::ResourcesState,
    cmd_stdout: cmd_stdout::ResourcesState,
}

impl ResourcesState {
    pub fn current(resources: &Resources) -> Result<Self> {
        Ok(Self {
            fs: fs::ResourcesState::current(&resources.paths)?,
            cmd_stdout: cmd_stdout::ResourcesState::current(&resources.cmds)?,
        })
    }

    pub fn eq_current_state(&self, resources: &Resources) -> Result<bool> {
        // TODO Parallelize this computation
        Ok((&self.fs).eq_current_state(&resources.paths)?
            && (&self.cmd_stdout).eq_current_state(&resources.cmds))
    }
}
