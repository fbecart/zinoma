use crate::domain::BuildTarget;
use crate::run_script;
use anyhow::{anyhow, Context, Result};
use async_std::prelude::*;
use async_std::sync::Receiver;
use futures::FutureExt;
use std::process::Stdio;
use std::time::Instant;

pub async fn build_target(
    target: &BuildTarget,
    mut build_cancellation_events: Receiver<BuildCancellationMessage>,
) -> Result<BuildCompletionReport> {
    let target_start = Instant::now();
    log::info!("{} - Building", target);

    let mut command = run_script::build_command(&target.build_script, &target.metadata.project_dir);
    command.stdout(Stdio::inherit()).stderr(Stdio::inherit());

    let mut build_process = command
        .spawn()
        .with_context(|| format!("Failed to spawn build command for {}", target))?;

    futures::select! {
        _ = build_cancellation_events.next().fuse() => {
            log::debug!("{} - Build cancelled", target);
            if let Err(e) = build_process.kill() {
                log::error!("{} - Failed to kill build process: {}", target, e)
            };
            if let Err(e) = build_process.status().await {
                log::error!("{} - Failed to await build process: {}", target, e)
            }
            Ok(BuildCompletionReport::Aborted)
        },
        result = build_process.status().fuse() => {
            let exit_status = result?;
            if !exit_status.success() {
                return Err(anyhow!("Build failed with {}", exit_status));
            }
            let target_build_duration = target_start.elapsed();
            log::info!(
                "{} - Build success (took: {}ms)",
                target,
                target_build_duration.as_millis()
            );
            Ok(BuildCompletionReport::Completed)
        },
    }
}

pub enum BuildCompletionReport {
    Completed,
    Aborted,
}

pub struct BuildCancellationMessage;
