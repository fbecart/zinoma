use crate::incremental::{IncrementalRunResult, IncrementalRunner};
use crate::target::{Target, TargetId};
use anyhow::{Context, Result};
use crossbeam::channel::Sender;
use crossbeam::thread::Scope;
use duct::cmd;
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
        .run(&target.name, &target.input_paths, || {
            let target_start = Instant::now();
            log::info!("{} - Building", &target.name);
            for command in target.build_list.iter() {
                let command_start = Instant::now();
                log::debug!("{} - Command \"{}\" - Executing", target.name, command);
                let command_output = cmd!("/bin/sh", "-c", command)
                    .dir(&target.path)
                    .stderr_to_stdout()
                    .run()
                    .with_context(|| "Command execution error")?;
                print!(
                    "{}",
                    String::from_utf8(command_output.stdout)
                        .with_context(|| "Failed to interpret stdout as utf-8")?
                );
                let command_execution_duration = command_start.elapsed();
                log::debug!(
                    "{} - Command \"{}\" - Success (took: {}ms)",
                    target.name,
                    command,
                    command_execution_duration.as_millis()
                );
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
