use crate::incremental::{IncrementalRunResult, IncrementalRunner};
use crate::target::{Target, TargetId};
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

    pub fn build(&'a self, scope: &Scope<'a>, target: &'a Target, tx: &Sender<BuildResult>) {
        let tx = tx.clone();
        scope.spawn(move |_| {
            build_target(target, &self.incremental_runner, &tx)
                .map_err(|e| format!("Error building target {}: {}", target.id, e))
                .unwrap()
        });
    }
}

pub fn build_target(
    target: &Target,
    incremental_runner: &IncrementalRunner,
    tx: &Sender<BuildResult>,
) -> Result<(), String> {
    let incremental_run_result = incremental_runner
        .run(&target.name, &target.watch_list, || {
            let target_start = Instant::now();
            log::info!("{} - Building", &target.name);
            for command in target.build_list.iter() {
                let command_start = Instant::now();
                log::debug!("{} - Command \"{}\" - Executing", target.name, command);
                let command_output = cmd!("/bin/sh", "-c", command)
                    .dir(&target.path)
                    .stderr_to_stdout()
                    .run()
                    .map_err(|e| format!("Command execution error: {}", e))?;
                print!(
                    "{}",
                    String::from_utf8(command_output.stdout)
                        .map_err(|e| format!("Failed to interpret stdout as utf-8: {}", e))?
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
        .map_err(|e| format!("Incremental build error: {}", e))?;

    if incremental_run_result == IncrementalRunResult::Skipped {
        log::info!("{} - Build skipped (Not Modified)", target.name);
    }

    let build_result_state = match incremental_run_result {
        IncrementalRunResult::Skipped => BuildResultState::Skip,
        IncrementalRunResult::Run(Ok(_)) => BuildResultState::Success,
        IncrementalRunResult::Run(Err(e)) => BuildResultState::Fail(e),
    };
    tx.send(BuildResult::new(target.id, build_result_state))
        .map_err(|e| format!("Sender error: {}", e))
}

pub struct BuildResult {
    pub target_id: TargetId,
    pub state: BuildResultState,
}

impl BuildResult {
    pub fn new(target_id: TargetId, state: BuildResultState) -> Self {
        Self { target_id, state }
    }
}

#[derive(Debug)]
pub enum BuildResultState {
    Success,
    Fail(String),
    Skip,
}
