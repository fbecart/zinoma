mod config;
mod incremental;
mod target;
mod watcher;

use crate::config::Config;
use crate::incremental::{IncrementalRunResult, IncrementalRunner};
use crate::target::{Target, TargetId};
use crate::watcher::TargetsWatcher;
use clap::{App, Arg};
use crossbeam::channel::{unbounded, Receiver, Sender, TryRecvError};
use duct::cmd;
use std::collections::HashMap;
use std::fmt;
use std::path::Path;
use std::thread::sleep;
use std::time::Duration;

fn main() -> Result<(), String> {
    let arg_matches = App::new("Buildy")
        .about("An ultra-fast parallel build system for local iteration")
        .arg(
            Arg::with_name("project_dir")
                .short("p")
                .long("project")
                .value_name("PROJECT_DIR")
                .help("Directory of the project to build (in which 'buildy.yml' is located)")
                .takes_value(true)
                .default_value("."),
        )
        .arg(
            Arg::with_name("targets")
                .value_name("TARGETS")
                .multiple(true)
                .required(true)
                .help("Targets to build"),
        )
        .get_matches();

    let project_dir = Path::new(arg_matches.value_of("project_dir").unwrap());
    let config_file_name = project_dir.join("buildy.yml");
    let requested_targets = arg_matches.values_of_lossy("targets").unwrap();
    let targets =
        Config::from_yml_file(&config_file_name)?.into_targets(&project_dir, &requested_targets)?;

    let checksum_dir = project_dir.join(".buildy");
    let incremental_runner = IncrementalRunner::new(&checksum_dir);

    Builder::new(project_dir, targets)
        .build_loop(&incremental_runner)
        .map_err(|e| format!("Build loop error: {}", e))?;
    // TODO: Detect cycles.
    Ok(())
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

#[derive(Clone)]
struct TargetBuildState {
    to_build: bool,
    being_built: bool,
    built: bool,
}

impl TargetBuildState {
    pub fn new() -> Self {
        Self {
            to_build: true,
            being_built: false,
            built: false,
        }
    }

    pub fn build_invalidated(&mut self) {
        self.to_build = true;
        self.built = false;
    }

    pub fn build_started(&mut self) {
        self.to_build = false;
        self.being_built = true;
        self.built = false;
    }

    pub fn build_finished(&mut self) {
        self.being_built = false;
        self.built = !self.to_build;
    }
}

struct Builder<'a> {
    project_dir: &'a Path,
    targets: Vec<Target>,
}

impl<'a> Builder<'a> {
    fn new(project_dir: &'a Path, targets: Vec<Target>) -> Self {
        Self {
            project_dir,
            targets,
        }
    }

    fn build_loop(&self, incremental_runner: &IncrementalRunner) -> Result<(), String> {
        /* Choose build targets (based on what's already been built, dependency tree, etc)
        Build all of them in parallel
        Wait for things to be built
        As things get built, check to see if there's something new we can build
        If so, start building that in parallel too

        Stop when nothing is still building and there's nothing left to build */
        crossbeam::scope(|scope| {
            let watcher = TargetsWatcher::new(&self.targets)
                .map_err(|e| format!("Failed to set up file watcher: {}", e))?;

            let mut target_build_states: Vec<TargetBuildState> =
                vec![TargetBuildState::new(); self.targets.len()];

            let (tx, rx) = unbounded();

            let mut run_tx_channels: HashMap<TargetId, Sender<RunSignal>> = Default::default();

            loop {
                for target_id in watcher
                    .get_invalidated_targets()
                    .map_err(|e| format!("File watch error: {}", e))?
                {
                    target_build_states[target_id].build_invalidated();
                }

                let ready_to_build: Vec<TargetId> = target_build_states
                    .iter()
                    .enumerate()
                    .filter(|(_target_id, build_state)| {
                        build_state.to_build && !build_state.being_built
                    })
                    .map(|(target_id, _build_state)| target_id)
                    .filter(|target_id| {
                        let target = self.targets.get(*target_id).unwrap();

                        target
                            .depends_on
                            .iter()
                            .all(|dependency_id| target_build_states[*dependency_id].built)
                    })
                    .collect();

                for target_id in ready_to_build.iter() {
                    let target = self.targets.get(*target_id).unwrap();
                    println!("Building {}", &target.name);
                    target_build_states[target.id].build_started();
                    let tx_clone = tx.clone();
                    scope.spawn(move |_| {
                        self.build(target.id, tx_clone, &incremental_runner)
                            .map_err(|e| format!("Error building target {}: {}", target.id, e))
                            .unwrap()
                    });
                }

                match rx.try_recv() {
                    Ok(result) => {
                        let target_id = result.target_id;
                        let target = self.targets.get(target_id).unwrap();

                        match result.state {
                            BuildResultState::Success => println!("DONE {}", target.name),
                            BuildResultState::Skip => {
                                println!("SKIP (Not Modified) {}", target.name)
                            }
                            BuildResultState::Fail => {
                                return Err(format!("Build failed for target {}", target.name));
                            }
                        }
                        target_build_states[target_id].build_finished();

                        // If already running, send a kill signal.
                        match run_tx_channels.get(&target_id) {
                            None => {}
                            Some(run_tx) => run_tx.send(RunSignal::Kill).map_err(|e| {
                                format!(
                                    "Failed to send run signal '{}' to running process: {}",
                                    RunSignal::Kill,
                                    e
                                )
                            })?,
                        }

                        if let Some(command) = &target.run {
                            let (run_tx, run_rx) = unbounded();
                            run_tx_channels.insert(target_id.to_owned(), run_tx);

                            let command = command.to_string();
                            scope.spawn(move |_| self.run(&command, run_rx).unwrap());
                        }
                    }
                    Err(TryRecvError::Empty) => {}
                    Err(e) => return Err(format!("Crossbeam parallelism failure: {}", e)),
                }

                sleep(Duration::from_millis(10))
            }
        })
        .map_err(|_| "Unknown crossbeam parallelism failure (thread panicked)".to_string())
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
                        .dir(&self.project_dir)
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

    fn run(&self, command: &str, rx: Receiver<RunSignal>) -> Result<(), String> {
        println!("Running command: {}", command);
        let handle = cmd!("/bin/sh", "-c", command)
            .dir(&self.project_dir)
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
