use crate::domain::BuildTarget;
use crate::run_script;
use anyhow::{anyhow, Context, Error, Result};
use async_std::prelude::*;
use async_std::sync::Receiver;
use async_std::task;
use futures::FutureExt;
use std::process::Stdio;
use std::time::Instant;

pub fn build_target(target: &BuildTarget, mut termination_events: Receiver<()>) -> Result<()> {
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

        futures::select! {
            result = build_process.fuse() => match result {
                Err(build_error) => {
                    Err(Error::new(build_error).context(format!("Build failed to run")))
                }
                Ok(exit_status) if !exit_status.success() => Err(anyhow!(
                    "{} - Build failed (exit status: {})",
                    target,
                    exit_status
                )),
                Ok(_exit_status) => {
                    let target_build_duration = target_start.elapsed();
                    log::info!(
                        "{} - Build success (took: {}ms)",
                        target,
                        target_build_duration.as_millis()
                    );
                    Ok(())
                }
            },
            _ = termination_events.next().fuse() => Err(anyhow!("{} - Build cancelled", target)),
        }
    })
}
