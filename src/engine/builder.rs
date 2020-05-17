use super::incremental::{IncrementalRunResult, IncrementalRunner};
use crate::domain::{Target, TargetId};
use anyhow::{Context, Result};
use crossbeam::channel::Sender;
use crossbeam::thread::Scope;
use run_script::{IoOptions, ScriptOptions};
use std::time::Instant;

pub struct TargetBuilder<'a> {
    incremental_runner: IncrementalRunner<'a>,
}

impl<'a> TargetBuilder<'a> {
    pub fn new(incremental_runner: IncrementalRunner<'a>) -> Self {
        Self { incremental_runner }
    }

    pub fn build(&'a self, scope: &Scope<'a>, target: &'a Target, tx: &Sender<BuildReport>) {
        let tx = tx.clone();
        scope.spawn(move |_| {
            build_target(target, &self.incremental_runner, &tx)
                .with_context(|| format!("Error building target {}", target.id))
                .unwrap()
        });
    }
}

pub fn build_target(
    target: &Target,
    incremental_runner: &IncrementalRunner,
    tx: &Sender<BuildReport>,
) -> Result<()> {
    let result = incremental_runner
        .run(&target, || {
            let target_start = Instant::now();

            if let Some(script) = &target.build {
                log::info!("{} - Building", &target.name);

                let mut options = ScriptOptions::new();
                options.exit_on_error = true;
                options.output_redirection = IoOptions::Inherit;
                options.working_directory = Some(target.path.to_path_buf());

                let (code, _output, _error) = run_script::run(&script, &vec![], &options)
                    .with_context(|| "Build execution error")?;

                if code != 0 {
                    return Err(anyhow::anyhow!("Build failed for target {}", target.name));
                }
            }

            let target_build_duration = target_start.elapsed();
            log::info!(
                "{} - Built (took: {}ms)",
                target.name,
                target_build_duration.as_millis()
            );
            Ok(())
        })
        .with_context(|| "Incremental build error")?;

    if let IncrementalRunResult::Skipped = result {
        log::info!("{} - Build skipped (Not Modified)", target.name);
    }

    tx.send(BuildReport::new(target.id, result))
        .with_context(|| "Sender error")
}

pub struct BuildReport {
    pub target_id: TargetId,
    pub result: IncrementalRunResult<Result<()>>,
}

impl BuildReport {
    pub fn new(target_id: TargetId, result: IncrementalRunResult<Result<()>>) -> Self {
        Self { target_id, result }
    }
}
