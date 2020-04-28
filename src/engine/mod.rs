mod build_state;
mod service;
mod watcher;

use crate::incremental::{IncrementalRunResult, IncrementalRunner};
use crate::target::Target;
use build_state::{BuildResult, BuildResultState, TargetBuildStates};
use crossbeam::channel::Sender;
use crossbeam::thread::Scope;
use duct::cmd;
use service::ServicesRunner;
use std::thread::sleep;
use std::time::{Duration, Instant};
use watcher::TargetsWatcher;

pub struct Engine<'a> {
    targets: Vec<Target>,
    incremental_runner: IncrementalRunner<'a>,
}

impl<'a> Engine<'a> {
    pub fn new(targets: Vec<Target>, incremental_runner: IncrementalRunner<'a>) -> Self {
        Self {
            targets,
            incremental_runner,
        }
    }

    pub fn watch(&'a self, scope: &Scope<'a>) -> Result<(), String> {
        let watcher = TargetsWatcher::new(&self.targets)
            .map_err(|e| format!("Failed to set up file watcher: {}", e))?;

        let mut services_runner = ServicesRunner::new(&self.targets);

        let mut target_build_states = TargetBuildStates::new(&self.targets);

        loop {
            let invalidated_builds = watcher
                .get_invalidated_targets()
                .map_err(|e| format!("File watch error: {}", e))?;
            target_build_states.set_builds_invalidated(&invalidated_builds);

            self.build_ready_targets(scope, &mut target_build_states);

            if let Some(result) = target_build_states.get_finished_build()? {
                let target = &self.targets[result.target_id];
                if let BuildResultState::Fail(e) = result.state {
                    log::warn!("{} - Build failed: {}", target.name, e);
                } else {
                    services_runner.restart_service(scope, target)?;
                }
            }

            sleep(Duration::from_millis(10))
        }
    }

    pub fn build(&'a self, scope: &Scope<'a>) -> Result<(), String> {
        let mut target_build_states = TargetBuildStates::new(&self.targets);

        loop {
            if target_build_states.all_are_built() {
                break Ok(());
            }

            self.build_ready_targets(scope, &mut target_build_states);

            if let Some(result) = target_build_states.get_finished_build()? {
                if let BuildResultState::Fail(e) = result.state {
                    let target = &self.targets[result.target_id];
                    return Err(format!("Build failed for target {}: {}", target.name, e));
                }
            }

            sleep(Duration::from_millis(10))
        }
    }

    fn build_ready_targets(
        &'a self,
        scope: &Scope<'a>,
        target_build_states: &mut TargetBuildStates,
    ) {
        for &target_id in target_build_states.get_ready_to_build_targets().iter() {
            let target = self.targets.get(target_id).unwrap();
            target_build_states.set_build_started(target.id);
            let tx = target_build_states.tx.clone();
            scope.spawn(move |_| {
                build_target(target, &self.incremental_runner, &tx)
                    .map_err(|e| format!("Error building target {}: {}", target.id, e))
                    .unwrap()
            });
        }
    }
}

fn build_target(
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
