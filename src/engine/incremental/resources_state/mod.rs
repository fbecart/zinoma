mod cmd_stdout;
mod fs;

use crate::domain::Resources;
use anyhow::Result;
use futures::future;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, PartialEq)]
pub struct ResourcesState {
    fs: fs::ResourcesState,
    cmd_stdout: cmd_stdout::ResourcesState,
}

impl ResourcesState {
    pub async fn current(resources: &Resources) -> Result<Self> {
        let (fs, cmd_stdout) = future::join(
            fs::ResourcesState::current(&resources.paths),
            cmd_stdout::ResourcesState::current(&resources.cmds),
        )
        .await;

        Ok(Self {
            fs: fs?,
            cmd_stdout: cmd_stdout?,
        })
    }

    pub async fn eq_current_state(&self, resources: &Resources) -> Result<bool> {
        // TODO Resolve at the first negative result
        let (fs, cmd_stdout) = future::join(
            self.fs.eq_current_state(&resources.paths),
            self.cmd_stdout.eq_current_state(&resources.cmds),
        )
        .await;

        Ok(fs && cmd_stdout)
    }
}
