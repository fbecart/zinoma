use crate::async_utils::ExponentialBackoff;
use crate::domain::BuildTarget;
use crate::run_script;
use anyhow::{anyhow, Context, Result};
use async_std::prelude::*;
use async_std::sync::Receiver;
use async_std::task;
use futures::FutureExt;
use std::process::Stdio;
use std::time::{Duration, Instant};

pub async fn build_target(
    target: &BuildTarget,
    mut build_cancellation_events: Receiver<BuildCancellationMessage>,
) -> Result<BuildCompletionReport> {
    let target_start = Instant::now();
    log::info!("{} - Building", target);

    let mut command = run_script::build_command(&target.build_script, &target.metadata.project_dir);
    command.stdout(Stdio::inherit()).stderr(Stdio::inherit());

    let mut build_process = task::spawn_blocking(move || command.spawn())
        .await
        .with_context(|| format!("Failed to spawn build command for {}", target))?;

    let mut ticks =
        ExponentialBackoff::new(Duration::from_millis(1), Duration::from_millis(100), 2.0);

    loop {
        futures::select! {
            _ = build_cancellation_events.next().fuse() => {
                log::debug!("{} - Build cancelled", target);
                if let Err(e) = task::spawn_blocking(move || build_process.kill().and_then(|_| build_process.wait())).await {
                    log::error!("{} - Failed to kill build process: {}", target, e)
                }
                break Ok(BuildCompletionReport::Aborted);
            },
            _ = ticks.next().fuse() => {
                if let Some(exit_status) = build_process.try_wait()? {
                    if !exit_status.success() {
                        break Err(anyhow!("Build failed with {}", exit_status));
                    }
                    let target_build_duration = target_start.elapsed();
                    log::info!(
                        "{} - Build success (took: {}ms)",
                        target,
                        target_build_duration.as_millis()
                    );
                    break Ok(BuildCompletionReport::Completed);
                }
            },
        }
    }
}

pub enum BuildCompletionReport {
    Completed,
    Aborted,
}

pub struct BuildCancellationMessage;
