use std::time::Instant;

use anyhow::{Context, Result};
use crossbeam::channel::{Receiver, tick};
use run_script::{IoOptions, ScriptOptions};
use std::time::Duration;

use crate::domain::Target;

pub fn build_target(target: &Target, termination_events: Receiver<()>) -> Result<()> {
    if let Some(script) = &target.build {
        let target_start = Instant::now();
        log::info!("{} - Building", &target.name);

        let mut options = ScriptOptions::new();
        options.exit_on_error = true;
        options.output_redirection = IoOptions::Inherit;
        options.working_directory = Some(target.path.to_path_buf());

        let mut process = run_script::spawn(&script, &vec![], &options)
            .with_context(|| "Build script execution error")?;

        let ticks = tick(Duration::from_millis(10));

        loop {
            crossbeam_channel::select! {
                recv(ticks) -> _ => {
                    if let Some(exit_status) = process.try_wait()? {
                        if !exit_status.success() {
                            return Err(anyhow::anyhow!("Build failed for target {} ({})", target.name, exit_status));
                        }
                        let target_build_duration = target_start.elapsed();
                        log::info!(
                            "{} - Built (took: {}ms)",
                            target.name,
                            target_build_duration.as_millis()
                        );
                        break;
                    }
                },
                recv(termination_events) -> _ => {
                    process.kill().with_context(|| format!("Failed to kill build process for {}", target.name))?;
                    return Err(anyhow::anyhow!("Build cancelled for target {}", target.name));
                },
            }
        }
    }

    Ok(())
}
