use std::time::Instant;

use anyhow::{Context, Result};
use run_script::{IoOptions, ScriptOptions};

use crate::domain::Target;

pub fn build_target(target: &Target) -> Result<()> {
    if let Some(script) = &target.build {
        let target_start = Instant::now();
        log::info!("{} - Building", &target.name);

        let mut options = ScriptOptions::new();
        options.exit_on_error = true;
        options.output_redirection = IoOptions::Inherit;
        options.working_directory = Some(target.path.to_path_buf());

        let (code, _output, _error) = run_script::run(&script, &vec![], &options)
            .with_context(|| "Script execution error")?;

        if code != 0 {
            return Err(anyhow::anyhow!("Build failed for target {}", target.name));
        }

        let target_build_duration = target_start.elapsed();
        log::info!(
            "{} - Built (took: {}ms)",
            target.name,
            target_build_duration.as_millis()
        );
    }

    Ok(())
}
