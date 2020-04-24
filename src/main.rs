mod config;
mod incremental;
mod target;
mod watcher;

use crate::config::Config;
use crate::incremental::{IncrementalRunResult, IncrementalRunner};
use crate::target::{Target, TargetId};
use crate::watcher::TargetsWatcher;
use clap::{App, Arg};
use crossbeam::channel::{unbounded, Receiver, SendError, Sender, TryRecvError};
use duct::cmd;
use std::collections::{HashMap, HashSet};
use std::fmt;
use std::path::Path;
use std::thread::sleep;
use std::time::Duration;

fn main() -> Result<(), String> {
    let arg_matches = App::new("Buildy")
        .about("An ultra-fast parallel build system for local iteration")
        .arg(
            Arg::with_name("config")
                .short("c")
                .long("config")
                .value_name("FILE")
                .help("Sets a custom config file")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("targets")
                .value_name("TARGETS")
                .multiple(true)
                .required(true)
                .help("Targets to build"),
        )
        .get_matches();

    let config_file_name = arg_matches.value_of("config").unwrap_or("buildy.yml");
    let requested_targets = arg_matches.values_of_lossy("targets").unwrap();
    let targets =
        Config::from_yml_file(Path::new(config_file_name))?.into_targets(&requested_targets)?;

    let incremental_runner = IncrementalRunner::new(Path::new(".buildy"));

    Builder::new(targets)
        .build_loop(&incremental_runner)
        .map_err(|e| format!("Build loop error: {}", e))?;
    // TODO: Detect cycles.
    Ok(())
}

enum BuildLoopError {
    BuildFailed(TargetId),
    UnspecifiedCrossbeamError,
    CrossbeamSendError(SendError<RunSignal>),
    CrossbeamRecvError(TryRecvError),
    WatcherSetupError(String),
    WatcherError(String),
}

impl fmt::Display for BuildLoopError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BuildLoopError::BuildFailed(target) => write!(f, "Build failed for target {}", target),
            BuildLoopError::UnspecifiedCrossbeamError => {
                write!(f, "Unknown crossbeam parallelism failure")
            }
            BuildLoopError::CrossbeamSendError(send_err) => write!(
                f,
                "Failed to send run signal '{}' to running process",
                send_err.0
            ),
            BuildLoopError::CrossbeamRecvError(recv_err) => {
                write!(f, "Crossbeam parallelism failure: {}", recv_err)
            }
            BuildLoopError::WatcherSetupError(notify_err) => {
                write!(f, "File watch error: {}", notify_err)
            }
            BuildLoopError::WatcherError(message) => write!(f, "File watch error: {}", message),
        }
    }
}

struct BuildResult {
    target_id: TargetId,
    state: BuildResultState,
}

#[derive(Debug)]
enum BuildResultState {
    Success,
    Fail,
    Skip,
}

enum RunSignal {
    Kill,
}

impl fmt::Display for RunSignal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RunSignal::Kill => write!(f, "KILL"),
        }
    }
}

struct Builder {
    targets: Vec<Target>,
}

impl Builder {
    fn new(targets: Vec<Target>) -> Self {
        Builder { targets }
    }

    fn is_target_to_build(
        target: &Target,
        built_targets: &HashSet<TargetId>,
        building: &HashSet<TargetId>,
        has_changed_files: &HashSet<TargetId>,
    ) -> bool {
        let dependencies_satisfied = target
            .depends_on
            .iter()
            .all(|dependency_id| built_targets.contains(dependency_id));

        if !dependencies_satisfied {
            return false;
        }
        if building.contains(&target.id) {
            return false;
        }
        if built_targets.contains(&target.id) {
            if !target.incremental_run {
                return false;
            }
            if !has_changed_files.contains(&target.id) {
                return false;
            }
        }

        true
    }

    fn build_loop(&self, incremental_runner: &IncrementalRunner) -> Result<(), BuildLoopError> {
        /* Choose build targets (based on what's already been built, dependency tree, etc)
        Build all of them in parallel
        Wait for things to be built
        As things get built, check to see if there's something new we can build
        If so, start building that in parallel too

        Stop when nothing is still building and there's nothing left to build */
        crossbeam::scope(|scope| {
            let watcher =
                TargetsWatcher::new(&self.targets).map_err(BuildLoopError::WatcherSetupError)?;

            let mut to_build: HashSet<TargetId> = HashSet::new();
            let mut has_changed_files: HashSet<TargetId> = HashSet::new();
            let mut built_targets: HashSet<TargetId> = HashSet::new();
            let mut building: HashSet<TargetId> = HashSet::new();

            let (tx, rx) = unbounded();

            let mut run_tx_channels: HashMap<TargetId, Sender<RunSignal>> = Default::default();

            loop {
                for target_id in watcher
                    .get_invalidated_targets()
                    .map_err(BuildLoopError::WatcherError)?
                {
                    has_changed_files.insert(target_id);
                }

                for target in self.targets.iter().filter(|target| {
                    Self::is_target_to_build(target, &built_targets, &building, &has_changed_files)
                }) {
                    to_build.insert(target.id);
                }

                // if self.to_build.len() == 0 && self.building.len() == 0 {
                //    TODO: Exit if nothing to watch.
                //    break;
                // }

                for target_id in to_build.iter() {
                    let target_id = *target_id;
                    let target = self.targets.get(target_id).unwrap();
                    println!("Building {}", &target.name);
                    building.insert(target_id);
                    has_changed_files.remove(&target_id);
                    let tx_clone = tx.clone();
                    scope.spawn(move |_| {
                        self.build(target_id, tx_clone, &incremental_runner)
                            .map_err(|e| format!("Error building target {}: {}", target_id, e))
                            .unwrap()
                    });
                }
                to_build.clear();

                match rx.try_recv() {
                    Ok(result) => {
                        let result_target_id = result.target_id;
                        self.parse_build_result(result, &mut building, &mut built_targets)?;

                        let target = self.targets.get(result_target_id).unwrap();

                        // If already running, send a kill signal.
                        match run_tx_channels.get(&result_target_id) {
                            None => {}
                            Some(run_tx) => run_tx
                                .send(RunSignal::Kill)
                                .map_err(BuildLoopError::CrossbeamSendError)?,
                        }

                        if let Some(command) = &target.run {
                            let (run_tx, run_rx) = unbounded();
                            run_tx_channels.insert(result_target_id.to_owned(), run_tx);

                            let command = command.to_string();
                            scope.spawn(move |_| Builder::run(&command, run_rx).unwrap());
                        }
                    }
                    Err(TryRecvError::Empty) => {}
                    Err(e) => return Err(BuildLoopError::CrossbeamRecvError(e)),
                }

                sleep(Duration::from_millis(10))
            }
        })
        .map_err(|_| BuildLoopError::UnspecifiedCrossbeamError)
        .and_then(|r| r)?;
        Ok(())
    }

    fn build(
        &self,
        target_id: TargetId,
        tx: Sender<BuildResult>,
        incremental_runner: &IncrementalRunner,
    ) -> Result<(), String> {
        let target = self.targets.get(target_id).unwrap();
        let incremental_run_result: IncrementalRunResult<Result<(), String>> = incremental_runner
            .run(&target.name, &target.watch_list, || {
                for command in target.build_list.iter() {
                    println!("Running build command: {}", command);
                    let command_output = cmd!("/bin/sh", "-c", command)
                        .stderr_to_stdout()
                        .run()
                        .map_err(|e| format!("Command execution error: {}", e))?;
                    println!(
                        "{}",
                        String::from_utf8(command_output.stdout)
                            .map_err(|e| format!("Failed to interpret stdout as utf-8: {}", e))?
                    );
                    println!("Ok {}", command);
                }
                Ok(())
            })
            .map_err(|e| format!("Incremental build error: {}", e))?;

        let build_result_state = match incremental_run_result {
            IncrementalRunResult::Skipped => BuildResultState::Skip,
            IncrementalRunResult::Run(Ok(_)) => BuildResultState::Success,
            IncrementalRunResult::Run(Err(_)) => BuildResultState::Fail,
        };
        tx.send(BuildResult {
            target_id: target.id,
            state: build_result_state,
        })
        .map_err(|e| format!("Sender error: {}", e))?;

        Ok(())
    }

    fn parse_build_result(
        &self,
        result: BuildResult,
        building: &mut HashSet<TargetId>,
        built_targets: &mut HashSet<TargetId>,
    ) -> Result<(), BuildLoopError> {
        match result.state {
            BuildResultState::Success => {
                let target = self.targets.get(result.target_id).unwrap();
                println!("DONE {}", target.name);
            }
            BuildResultState::Fail => {
                return Err(BuildLoopError::BuildFailed(result.target_id));
            }
            BuildResultState::Skip => {
                let target = self.targets.get(result.target_id).unwrap();
                println!("SKIP (Not Modified) {}", target.name);
            }
        }
        building.remove(&result.target_id);
        built_targets.insert(result.target_id);
        Ok(())
    }

    fn run(command: &str, rx: Receiver<RunSignal>) -> Result<(), String> {
        println!("Running command: {}", command);
        let handle = cmd!("/bin/sh", "-c", command)
            .stderr_to_stdout()
            .start()
            .map_err(|e| format!("Failed to run command {}: {}", command, e))?;

        match rx.recv() {
            Ok(RunSignal::Kill) => handle
                .kill()
                .map_err(|e| format!("Failed to kill process {}: {}", command, e)),
            Err(e) => Err(format!("Receiver error: {}", e)),
        }
    }
}
