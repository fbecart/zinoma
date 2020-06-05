use std::time::Instant;

use super::process;
use crate::run_script;
use anyhow::{anyhow, Context, Result};
use crossbeam::channel::{tick, Receiver};
use std::process::Stdio;
use std::time::Duration;

use crate::domain::Target;

pub fn build_target(target: &Target, termination_events: Receiver<()>) -> Result<()> {
    if let Some(script) = &target.build() {
        let target_start = Instant::now();
        log::info!("{} - Building", target);

        let mut build_process = run_script::build_command(script, &target.project.dir)
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .spawn()
            .with_context(|| format!("Failed to spawn build command for {}", target))?;

        let ticks = tick(Duration::from_millis(10));

        loop {
            crossbeam_channel::select! {
                recv(ticks) -> _ => {
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
                recv(termination_events) -> _ => {
                    process::kill_and_wait(&mut build_process).with_context(|| format!("Failed to kill build process for {}", target))?;
                    return Err(anyhow!("Build cancelled for target {}", target));
                },
            }
        }
    }

    Ok(())
}
