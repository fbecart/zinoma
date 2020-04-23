mod incremental;

use clap::{App, Arg};
use crossbeam;
use crossbeam::channel::{unbounded, Receiver, SendError, Sender, TryRecvError};
use duct::cmd;
use incremental::{IncrementalRunResult, IncrementalRunner};
use notify::{RawEvent, RecommendedWatcher, RecursiveMode, Watcher};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::env::current_dir;
use std::fmt;
use std::fs;
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

    let file_name = arg_matches.value_of("config").unwrap_or("buildy.yml");
    let contents = fs::read_to_string(file_name)
        .map_err(|e| format!("Something went wrong reading {}: {}", file_name, e))?;
    let targets: HashMap<String, Target> = serde_yaml::from_str(&contents)
        .map_err(|e| format!("Invalid format for {}: {}", file_name, e))?;
    check_targets(&targets).map_err(|e| format!("Failed sanity check: {}", e))?;

    let requested_targets = arg_matches.values_of_lossy("targets").unwrap();
    let invalid_targets: Vec<String> = requested_targets
        .iter()
        .filter(|requested_target| !targets.contains_key(*requested_target))
        .map(|i| i.to_owned())
        .collect();
    if !invalid_targets.is_empty() {
        return Err(format!("Invalid targets: {}", invalid_targets.join(", ")));
    }
    let targets = filter_targets(targets, requested_targets);

    let incremental_runner = IncrementalRunner::new(".buildy".to_string());

    Builder::new(targets)
        .build_loop(&incremental_runner)
        .map_err(|e| format!("Build loop error: {}", e))?;
    // TODO: Detect cycles.
    Ok(())
}

enum BuildLoopError {
    BuildFailed(String),
    UnspecifiedCrossbeamError,
    CrossbeamSendError(SendError<RunSignal>),
    CrossbeamRecvError(TryRecvError),
    WatcherError(notify::Error),
    CwdIOError(std::io::Error),
    CwdUtf8Error,
}

impl fmt::Display for BuildLoopError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BuildLoopError::BuildFailed(target) => write!(f, "Build failed for target {}", target),
            BuildLoopError::UnspecifiedCrossbeamError => {
                write!(f, "Unknown crossbeam parllelism failure")
            }
            BuildLoopError::CrossbeamSendError(send_err) => write!(
                f,
                "Failed to send run signal '{}' to running process",
                send_err.0
            ),
            BuildLoopError::CrossbeamRecvError(recv_err) => {
                write!(f, "Crossbeam parallelism failure: {}", recv_err)
            }
            BuildLoopError::WatcherError(notify_err) => {
                write!(f, "File watch error: {}", notify_err)
            }
            BuildLoopError::CwdIOError(io_err) => {
                write!(f, "IO Error while getting current directory: {}", io_err)
            }
            BuildLoopError::CwdUtf8Error => write!(f, "Current directory was not valid utf-8"),
        }
    }
}

struct BuildResult {
    target: String,
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
    targets: HashMap<String, Target>,
}

impl Builder {
    fn new(targets: HashMap<String, Target>) -> Self {
        Builder { targets }
    }

    fn is_target_to_build(
        target_name: &str,
        target: &Target,
        built_targets: &HashSet<String>,
        building: &HashSet<String>,
        has_changed_files: &HashSet<String>,
    ) -> bool {
        let dependencies_satisfied = target
            .depends_on
            .iter()
            .all(|dependency| built_targets.contains(dependency.as_str()));

        if !dependencies_satisfied {
            return false;
        }
        if building.contains(target_name) {
            return false;
        }
        if built_targets.contains(target_name) {
            if !target.run_options.incremental {
                return false;
            }
            if !has_changed_files.contains(target_name) {
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
            let (_watcher, watcher_rx) = self.setup_watcher()?;

            let mut to_build = HashSet::new();
            let mut has_changed_files = HashSet::new();
            let mut built_targets = HashSet::new();
            let mut building = HashSet::new();

            let (tx, rx) = unbounded();
            let working_dir = current_dir().map_err(BuildLoopError::CwdIOError)?;
            let working_dir = working_dir
                .to_str()
                .ok_or_else(|| BuildLoopError::CwdUtf8Error)?;

            let mut run_tx_channels: HashMap<String, Sender<RunSignal>> = Default::default();

            loop {
                match watcher_rx.try_recv() {
                    Ok(result) => {
                        let absolute_path = match result.path {
                            Some(path) => path,
                            None => continue,
                        };
                        let absolute_path = match absolute_path.to_str() {
                            Some(s) => s,
                            None => continue,
                        };

                        // TODO: This won't work with symlinks.
                        let relative_path = &absolute_path[working_dir.len() + 1..];

                        for (target_name, target) in self.targets.iter() {
                            if target
                                .watch_list
                                .iter()
                                .any(|watch_path| relative_path.starts_with(watch_path))
                            {
                                has_changed_files.insert(target_name.to_string());
                            }
                        }
                    }
                    Err(e) => match e {
                        TryRecvError::Empty => {}
                        _ => return Err(BuildLoopError::CrossbeamRecvError(e)),
                    },
                }

                self.targets
                    .iter()
                    .filter(|(target_name, target)| {
                        Self::is_target_to_build(
                            target_name,
                            target,
                            &built_targets,
                            &building,
                            &has_changed_files,
                        )
                    })
                    .for_each(|(target_name, _target)| {
                        to_build.insert(target_name.to_string());
                    });

                // if self.to_build.len() == 0 && self.building.len() == 0 {
                //    TODO: Exit if nothing to watch.
                //    break;
                // }

                for target_to_build in to_build.iter() {
                    let target_to_build = target_to_build.clone();
                    println!("Building {}", target_to_build);
                    building.insert(target_to_build.to_string());
                    has_changed_files.remove(&target_to_build);
                    let tx_clone = tx.clone();
                    let target = self.targets.get(target_to_build.as_str()).unwrap().clone();
                    scope.spawn(move |_| {
                        target
                            .build(&target_to_build, tx_clone, &incremental_runner)
                            .map_err(|e| {
                                format!("Error building target {}: {}", target_to_build, e)
                            })
                            .unwrap()
                    });
                }
                to_build.clear();

                match rx.try_recv() {
                    Ok(result) => {
                        let result_target = result.target.to_string();
                        self.parse_build_result(result, &mut building, &mut built_targets)?;

                        let target = self.targets.get(&result_target).unwrap().clone();

                        // If already running, send a kill signal.
                        match run_tx_channels.get(&result_target) {
                            None => {}
                            Some(run_tx) => run_tx
                                .send(RunSignal::Kill)
                                .map_err(BuildLoopError::CrossbeamSendError)?,
                        }

                        if !target.run_list.is_empty() {
                            let (run_tx, run_rx) = unbounded();
                            run_tx_channels.insert(result_target.to_owned(), run_tx);

                            scope.spawn(move |_| {
                                target
                                    .run(run_rx)
                                    .map_err(|e| {
                                        format!("Error running target {}: {}", &result_target, e)
                                    })
                                    .unwrap()
                            });
                        }
                    }
                    Err(e) => {
                        if e != TryRecvError::Empty {
                            return Err(BuildLoopError::CrossbeamRecvError(e));
                        }
                    }
                }

                sleep(Duration::from_millis(10))
            }
        })
        .map_err(|_| BuildLoopError::UnspecifiedCrossbeamError)
        .and_then(|r| r)?;
        Ok(())
    }

    fn setup_watcher(&self) -> Result<(RecommendedWatcher, Receiver<RawEvent>), BuildLoopError> {
        let (watcher_tx, watcher_rx) = unbounded();
        let mut watcher: RecommendedWatcher =
            Watcher::new_immediate(watcher_tx).map_err(BuildLoopError::WatcherError)?;
        for target in self.targets.values() {
            for watch_path in target.watch_list.iter() {
                watcher
                    .watch(watch_path, RecursiveMode::Recursive)
                    .map_err(BuildLoopError::WatcherError)?;
            }
        }

        Ok((watcher, watcher_rx))
    }

    fn parse_build_result(
        &self,
        result: BuildResult,
        building: &mut HashSet<String>,
        built_targets: &mut HashSet<String>,
    ) -> Result<(), BuildLoopError> {
        match result.state {
            BuildResultState::Success => {
                println!("DONE {}", result.target);
            }
            BuildResultState::Fail => {
                return Err(BuildLoopError::BuildFailed(result.target));
            }
            BuildResultState::Skip => {
                println!("SKIP (Not Modified) {}", result.target);
            }
        }
        building.remove(&result.target);
        built_targets.insert(result.target);
        Ok(())
    }
}

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone)]
struct Target {
    #[serde(default)]
    depends_on: Vec<String>,
    #[serde(default, rename = "watch")]
    watch_list: Vec<String>,
    #[serde(default, rename = "build")]
    build_list: Vec<String>,
    #[serde(default, rename = "run")]
    run_list: Vec<String>,
    #[serde(default)]
    run_options: RunOptions,
}

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone)]
struct RunOptions {
    #[serde(default)]
    incremental: bool,
}

impl Default for RunOptions {
    fn default() -> Self {
        RunOptions { incremental: true }
    }
}

impl Target {
    fn build(
        &self,
        name: &str,
        tx: Sender<BuildResult>,
        incremental_runner: &IncrementalRunner,
    ) -> Result<(), String> {
        let incremental_run_result: IncrementalRunResult<Result<(), String>> = incremental_runner
            .run(name, &self.watch_list, || {
                for command in self.build_list.iter() {
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
            target: name.to_string(),
            state: build_result_state,
        })
        .map_err(|e| format!("Sender error: {}", e))?;

        Ok(())
    }

    fn run(&self, rx: Receiver<RunSignal>) -> Result<(), String> {
        for command in self.run_list.iter() {
            println!("Running command: {}", command);
            let handle = cmd!("/bin/sh", "-c", command)
                .stderr_to_stdout()
                .start()
                .map_err(|e| format!("Failed to run command {}: {}", command, e))?;
            loop {
                match rx.recv() {
                    Ok(RunSignal::Kill) => {
                        return handle
                            .kill()
                            .map_err(|e| format!("Failed to kill process {}: {}", command, e));
                    }
                    Err(e) => return Err(format!("Receiver error: {}", e)),
                }
            }
        }
        Ok(())
    }
}

enum TargetsCheckError<'a> {
    DependencyNotFound(&'a str),
    DependencyLoop(Vec<&'a str>),
}

impl fmt::Display for TargetsCheckError<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TargetsCheckError::DependencyNotFound(dependency) => {
                write!(f, "Dependency {} not found.", dependency)
            }
            TargetsCheckError::DependencyLoop(dependencies) => {
                write!(f, "Dependency loop: [{}]", dependencies.join(", "))
            }
        }
    }
}

fn check_targets(targets: &HashMap<String, Target>) -> Result<(), TargetsCheckError> {
    for (target_name, target) in targets.iter() {
        for dependency in target.depends_on.iter() {
            if !targets.contains_key(dependency.as_str()) {
                return Err(TargetsCheckError::DependencyNotFound(dependency));
            }
            if target_name == dependency {
                return Err(TargetsCheckError::DependencyLoop(vec![target_name]));
            }
        }
    }
    Ok(())
}

fn filter_targets(
    all_targets: HashMap<String, Target>,
    requested_targets: Vec<String>,
) -> HashMap<String, Target> {
    let mut filtered_targets: HashMap<String, Target> = HashMap::new();

    fn add_target(
        mut filtered_targets: &mut HashMap<String, Target>,
        all_targets: &HashMap<String, Target>,
        target_name: &str,
    ) {
        if filtered_targets.contains_key(target_name) {
            return;
        }

        let target = all_targets.get(target_name).unwrap();
        target
            .depends_on
            .iter()
            .for_each(|dependency| add_target(&mut filtered_targets, all_targets, dependency));
        filtered_targets.insert(target_name.to_owned(), target.clone());
    };

    requested_targets.iter().for_each(|requested_target| {
        add_target(&mut filtered_targets, &all_targets, requested_target)
    });

    filtered_targets
}
