use crossbeam;
use crossbeam::channel::{unbounded, Receiver, Sender, TryRecvError};
use crypto::digest::Digest;
use crypto::sha1::Sha1;
use duct::cmd;
use notify::{RawEvent, RecommendedWatcher, RecursiveMode, Watcher};
use serde::{Deserialize, Serialize};
use std::any::Any;
use std::collections::{HashMap, HashSet};
use std::env::current_dir;
use std::fs;
use std::io::Write;
use walkdir::WalkDir;

fn main() -> Result<(), String> {
    let file_name = ".buildy.yml";
    let contents = fs::read_to_string(file_name)
        .map_err(|_| format!("Something went wrong reading {}.", file_name))?;

    let targets: HashMap<String, Target> = serde_yaml::from_str(&contents)
        .map_err(|_| format!("Invalid format for {}.", file_name))?;
    let builder = Builder::new(targets);

    builder.sanity_check()?;
    builder
        .build_loop()
        .map_err(|_| String::from("Failed to start build loop"))?;
    // TODO: Detect cycles.
    Ok(())
}

struct BuildResult<'a> {
    target: &'a String,
    state: BuildResultState,
    output: String,
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

struct Builder {
    targets: HashMap<String, Target>,
}

impl Builder {
    fn new(targets: HashMap<String, Target>) -> Self {
        Builder { targets }
    }

    fn sanity_check(&self) -> Result<(), String> {
        for (target_name, target) in self.targets.iter() {
            for dependency in target.depends_on.iter() {
                if !self.targets.contains_key(dependency.as_str()) {
                    return Err(format!("Dependency {} not found.", dependency));
                }
                if target_name == dependency {
                    return Err(format!("Target {} cannot depend on itself.", target_name));
                }
            }
        }
        Ok(())
    }

    fn choose_build_targets<'a>(
        &'a self,
        built_targets: &mut HashSet<&'a String>,
        building: &mut HashSet<&'a String>,
        has_changed_files: &mut HashSet<&'a String>,
        to_build: &mut HashSet<&'a String>,
    ) {
        for (target_name, target) in self.targets.iter() {
            let dependencies_satisfied = target
                .depends_on
                .iter()
                .all(|dependency| built_targets.contains(dependency));

            if !dependencies_satisfied {
                continue;
            }
            if building.contains(target_name) {
                continue;
            }
            if built_targets.contains(target_name) {
                if !target.run_options.incremental {
                    continue;
                }
                if !has_changed_files.contains(target_name) {
                    continue;
                }
            }

            to_build.insert(target_name);
        }
    }

    fn build_loop(&self) -> Result<(), Box<dyn Any + Send>> {
        /* Choose build targets (based on what's already been built, dependency tree, etc)
        Build all of them in parallel
        Wait for things to be built
        As things get built, check to see if there's something new we can build
        If so, start building that in parallel too

        Stop when nothing is still building and there's nothing left to build */
        crossbeam::scope(|scope| {
            let (_watcher, watcher_rx) = self.setup_watcher().unwrap();

            let mut to_build = HashSet::new();
            let mut has_changed_files = HashSet::new();
            let mut built_targets = HashSet::new();
            let mut building = HashSet::new();

            let (tx, rx) = unbounded();
            let working_dir = current_dir().unwrap();
            let working_dir = working_dir.to_str().unwrap();

            let mut run_tx_channels: HashMap<&String, Sender<RunSignal>> = Default::default();

            loop {
                match watcher_rx.try_recv() {
                    Ok(result) => {
                        let absolute_path = result.path.unwrap();
                        let absolute_path = absolute_path.to_str().unwrap();

                        // TODO: This won't work with symlinks.
                        let relative_path = &absolute_path[working_dir.len() + 1..];

                        for (target_name, target) in self.targets.iter() {
                            if target
                                .watch_list
                                .iter()
                                .any(|watch_path| relative_path.starts_with(watch_path))
                            {
                                has_changed_files.insert(target_name);
                            }
                        }
                    }
                    Err(e) => {
                        if e != TryRecvError::Empty {
                            panic!("{}", e);
                        }
                    }
                }

                self.choose_build_targets(
                    &mut built_targets,
                    &mut building,
                    &mut has_changed_files,
                    &mut to_build,
                );

                // if self.to_build.len() == 0 && self.building.len() == 0 {
                //    TODO: Exit if nothing to watch.
                //    break;
                // }

                for target_to_build in to_build.iter() {
                    let target_to_build = target_to_build.clone();
                    println!("Building {}", target_to_build);
                    building.insert(target_to_build);
                    has_changed_files.remove(&target_to_build);
                    let tx_clone = tx.clone();
                    let target = self.targets.get(target_to_build).unwrap().clone();
                    scope.spawn(move |_| target.build(&target_to_build, tx_clone));
                }
                to_build.clear();

                match rx.try_recv() {
                    Ok(result) => {
                        self.parse_build_result(&result, &mut building, &mut built_targets);

                        let target = self.targets.get(result.target).unwrap().clone();

                        // If already running, send a kill signal.
                        match run_tx_channels.get(result.target) {
                            None => {}
                            Some(run_tx) => run_tx.send(RunSignal::Kill).unwrap(),
                        }

                        let (run_tx, run_rx) = unbounded();
                        run_tx_channels.insert(result.target, run_tx);

                        let tx_clone = tx.clone();
                        scope.spawn(move |_| target.run(tx_clone, run_rx));
                    }
                    Err(e) => {
                        if e != TryRecvError::Empty {
                            panic!("{}", e);
                        }
                    }
                }
            }
        })?;
        Ok(())
    }

    fn setup_watcher(&self) -> notify::Result<(RecommendedWatcher, Receiver<RawEvent>)> {
        let (watcher_tx, watcher_rx) = unbounded();
        let mut watcher: RecommendedWatcher = Watcher::new_immediate(watcher_tx)?;
        for target in self.targets.values() {
            for watch_path in target.watch_list.iter() {
                watcher.watch(watch_path, RecursiveMode::Recursive)?;
            }
        }

        Ok((watcher, watcher_rx))
    }

    fn parse_build_result<'a>(
        &'a self,
        result: &BuildResult<'a>,
        building: &mut HashSet<&'a String>,
        built_targets: &mut HashSet<&'a String>,
    ) {
        match result.state {
            BuildResultState::Success => {
                println!("DONE {}:\n{}", result.target, result.output);
            }
            BuildResultState::Fail => {
                panic!("Failed build: {}", result.target);
            }
            BuildResultState::Skip => {
                println!("SKIP (Not Modified) {}:\n{}", result.target, result.output);
            }
        }
        building.remove(result.target);
        built_targets.insert(result.target);
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
        return RunOptions { incremental: true };
    }
}

impl Target {
    fn build<'a>(&self, name: &'a String, tx: Sender<BuildResult<'a>>) -> Result<(), String> {
        let mut output_string = String::from("");

        let mut hasher = Sha1::new();

        if !self.watch_list.is_empty() {
            for path in self.watch_list.iter() {
                let checksum = calculate_checksum(path).map_err(ToOwned::to_owned)?;
                hasher.input_str(&checksum);
            }

            let watch_checksum = hasher.result_str();
            if does_checksum_match(name, &watch_checksum) {
                tx.send(BuildResult {
                    target: name,
                    state: BuildResultState::Skip,
                    output: output_string,
                })
                .unwrap();
                return Ok(());
            }
            write_checksum(name, &watch_checksum)?;
        }

        for command in self.build_list.iter() {
            println!("Running build command: {}", command);
            match cmd!("/bin/sh", "-c", command).stderr_to_stdout().run() {
                Ok(output) => {
                    println!("Ok {}", command);
                    output_string.push_str(
                        String::from_utf8(output.stdout)
                            .map_err(|_| String::from("Failed to interpret stdout as utf-8"))?
                            .as_str(),
                    );
                }
                Err(e) => {
                    println!("Err {} {}", e, command);
                    tx.send(BuildResult {
                        target: name,
                        state: BuildResultState::Fail,
                        output: output_string,
                    })
                    .unwrap();
                    return Ok(());
                }
            }
        }

        tx.send(BuildResult {
            target: name,
            state: BuildResultState::Success,
            output: output_string,
        })
        .unwrap();
        Ok(())
    }

    fn run(&self, _tx: Sender<BuildResult>, rx: Receiver<RunSignal>) {
        for command in self.run_list.iter() {
            println!("Running command: {}", command);
            match cmd!("/bin/sh", "-c", command).stderr_to_stdout().start() {
                Ok(handle) => loop {
                    match rx.recv() {
                        Ok(signal) => match signal {
                            RunSignal::Kill => {
                                match handle.kill() {
                                    Ok(_) => {}
                                    Err(e) => panic!("{}", e),
                                }
                                return;
                            }
                        },
                        Err(e) => panic!("RUN PANIC {}", e),
                    }
                },
                Err(e) => panic!("{}", e),
            }
        }
    }
}

fn calculate_checksum(path: &String) -> Result<String, &'static str> {
    let mut hasher = Sha1::new();

    for entry in WalkDir::new(path) {
        let entry = entry.map_err(|_| "Failed to traverse directory")?;

        if entry.path().is_file() {
            let entry_path = match entry.path().to_str() {
                Some(s) => s,
                None => return Err("Failed to convert file path into String"),
            };
            let contents =
                fs::read(entry_path).map_err(|_| "Failed to read file to calculate checksum")?;
            hasher.input(contents.as_slice());
        }
    }

    return Ok(hasher.result_str());
}

const CHECKSUM_DIRECTORY: &'static str = ".buildy";

fn checksum_file_name(target: &String) -> String {
    return format!("{}/{}.checksum", CHECKSUM_DIRECTORY, target);
}

fn does_checksum_match(target: &String, checksum: &String) -> bool {
    // Might want to check for some errors like permission denied.
    fs::create_dir(CHECKSUM_DIRECTORY).ok();
    let file_name = checksum_file_name(target);
    match fs::read_to_string(&file_name) {
        Ok(old_checksum) => {
            return *checksum == old_checksum;
        }
        Err(e) => {
            if e.kind() == std::io::ErrorKind::NotFound {
                // No checksum found.
                return false;
            }
            panic!(
                "Failed reading checksum file {} for target {}: {}",
                file_name, target, e
            );
        }
    };
}

fn write_checksum(target: &String, checksum: &String) -> Result<(), String> {
    let file_name = checksum_file_name(target);
    let mut file = fs::File::create(&file_name).map_err(|_| {
        format!(
            "Failed to create checksum file {} for target {}",
            file_name, target
        )
    })?;
    file.write_all(checksum.as_bytes()).map_err(|_| {
        format!(
            "Failed to write checksum file {} for target {}",
            file_name, target
        )
    })?;
    Ok(())
}
