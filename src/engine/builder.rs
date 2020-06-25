use crate::domain::BuildTarget;
use crate::run_script;
use anyhow::{anyhow, Context, Result};
use async_std::prelude::*;
use async_std::stream;
use async_std::sync::Receiver;
use futures::FutureExt;
use std::process::Stdio;
use std::time::{Duration, Instant};

pub async fn build_target(
    target: &BuildTarget,
    mut termination_events: Receiver<()>,
) -> Result<()> {
    let target_start = Instant::now();
    log::info!("{} - Building", target);

    let mut build_process =
        run_script::build_command(&target.build_script, &target.metadata.project_dir)
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .spawn()
            .with_context(|| format!("Failed to spawn build command for {}", target))?;

    let mut ticks = stream::interval(Duration::from_millis(10));

    loop {
        futures::select! {
            _ = ticks.next().fuse() => {
                if let Some(exit_status) = build_process.try_wait()? {
                    if !exit_status.success() {
                        return Err(anyhow!("Build failed for target {} ({})", target, exit_status));
                    }
                    let target_build_duration = target_start.elapsed();
                    log::info!(
                        "{} - Build success (took: {}ms)",
                        target,
                        target_build_duration.as_millis()
                    );
                    break;
                }
            },
            _ = termination_events.next().fuse() => {
                build_process.kill()
                    .and_then(|_| build_process.wait())
                    .with_context(|| format!("Failed to kill build process for {}", target))?;
                return Err(anyhow!("Build cancelled for target {}", target));
            },
        }
    }

    Ok(())
}
