mod build_state;
mod watcher;

use crate::incremental::{IncrementalRunResult, IncrementalRunner};
use crate::target::Target;
use build_state::{BuildResult, BuildResultState, TargetBuildStates};
use crossbeam::channel::{unbounded, Receiver, Sender};
use crossbeam::thread::Scope;
use duct::cmd;
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
        /* Choose build targets (based on what's already been built, dependency tree, etc)
        Build all of them in parallel
        Wait for things to be built
        As things get built, check to see if there's something new we can build
        If so, start building that in parallel too */

        let watcher = TargetsWatcher::new(&self.targets)
            .map_err(|e| format!("Failed to set up file watcher: {}", e))?;

        let mut service_tx_channels: Vec<Option<Sender<RunSignal>>> =
            vec![None; self.targets.len()];

        let mut target_build_states = TargetBuildStates::new(&self.targets);

        loop {
            for target_id in watcher
                .get_invalidated_targets()
                .map_err(|e| format!("File watch error: {}", e))?
            {
                target_build_states.set_build_invalidated(target_id);
            }

            self.build_ready_targets(scope, &mut target_build_states);

            if let Some(result) = target_build_states.get_finished_build()? {
                let target = &self.targets[result.target_id];
                if let BuildResultState::Fail(e) = result.state {
                    log::warn!("{} - Build failed: {}", target.name, e);
                } else if target.service.is_some() {
                    // If already running, send a kill signal.
                    if let Some(service_tx) = &service_tx_channels[target.id] {
                        service_tx.send(RunSignal::Kill).map_err(|e| {
                            format!("Failed to send Kill signal to running process: {}", e)
                        })?;
                    }

                    let (service_tx, service_rx) = unbounded();
                    service_tx_channels[target.id] = Some(service_tx);

                    scope.spawn(move |_| run_target_service(target, service_rx).unwrap());
                }
            }

            sleep(Duration::from_millis(10))
        }
    }

    pub fn build(&'a self, scope: &Scope<'a>) -> Result<(), String> {
        /* Choose build targets (based on what's already been built, dependency tree, etc)
        Build all of them in parallel
        Wait for things to be built
        As things get built, check to see if there's something new we can build
        If so, start building that in parallel too

        Stop when nothing is still building and there's nothing left to build */

        let mut target_build_states = TargetBuildStates::new(&self.targets);

        loop {
            if target_build_states.all_are_built() {
                break Ok(());
            }

            self.build_ready_targets(scope, &mut target_build_states);

            if let Some(result) = target_build_states.get_finished_build()? {
                let target = &self.targets[result.target_id];

                if let BuildResultState::Fail(e) = result.state {
                    return Err(format!("Build failed for target {}: {}", target.name, e));
                }
                target_build_states.set_build_succeeded(target.id);
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

fn run_target_service(target: &Target, rx: Receiver<RunSignal>) -> Result<(), String> {
    if let Some(command) = &target.service {
        log::info!("{} - Command: \"{}\" - Run", target.name, command);
        let handle = cmd!("/bin/sh", "-c", command)
            .dir(&target.path)
            .stderr_to_stdout()
            .start()
            .map_err(|e| format!("Failed to run command {}: {}", command, e))?;

        match rx.recv() {
            Ok(RunSignal::Kill) => {
                log::trace!("{} - Killing process", target.name);
                handle
                    .kill()
                    .map_err(|e| format!("Failed to kill process {}: {}", command, e))
            }
            Err(e) => Err(format!("Receiver error: {}", e)),
        }?
    }

    Ok(())
}

enum RunSignal {
    Kill,
}
