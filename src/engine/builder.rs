use std::time::Instant;

use crate::receiver::Receiver;
use crate::run_script;
use anyhow::{anyhow, Context, Error, Result};
use async_std::prelude::*;
use async_std::task;
use std::process::Stdio;

use crate::domain::BuildTarget;

pub fn build_target(target: &BuildTarget, termination_events: Receiver<()>) -> Result<()> {
    let target_start = Instant::now();
    log::info!("{} - Building", target);

    task::block_on(async {
        let build_process =
            run_script::build_command(&target.build_script, &target.metadata.project_dir)
                .stdout(Stdio::inherit())
                .stderr(Stdio::inherit())
                .kill_on_drop(true)
                .spawn()
                .with_context(|| format!("Failed to spawn build command for {}", target))?;

        let build_completed = async { Some(build_process.await) };
        let termination_events = async {
            let _ = termination_events.await;
            None
        };

        match build_completed.race(termination_events).await {
            None => Err(anyhow!("{} - Build cancelled", target)),
            Some(Err(build_error)) => {
                Err(Error::new(build_error).context(format!("Build failed to run")))
            }
            Some(Ok(exit_status)) if !exit_status.success() => Err(anyhow!(
                "{} - Build failed (exit status: {})",
                target,
                exit_status
            )),
            Some(Ok(_exit_status)) => {
                let target_build_duration = target_start.elapsed();
                log::info!(
                    "{} - Build success (took: {}ms)",
                    target,
                    target_build_duration.as_millis()
                );
                Ok(())
            }
        }
    })
}
