use crossbeam;
use crossbeam::channel::{unbounded, Receiver, Sender, TryRecvError};
use crypto::digest::Digest;
use crypto::sha1::Sha1;
use duct::cmd;
use duct::unix::HandleExt;
use notify::{RawEvent, RecommendedWatcher, RecursiveMode, Watcher};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::env::current_dir;
use std::fs;
use std::io::Write;
use walkdir::WalkDir;

fn main() {
    let file_name = ".buildy.yml";
    let contents = fs::read_to_string(file_name)
        .expect(&format!("Something went wrong reading {}.", file_name));

    let mut targets: HashMap<String, Target> =
        serde_yaml::from_str(&contents).expect(&format!("Invalid format for {}.", file_name));
    for (target_name, target) in targets.iter_mut() {
        target.name = target_name.clone();
    }
    let mut builder = Builder::new(targets);

    builder.sanity_check();
    builder.build_loop();
    // TODO: Detect cycles.
}

struct BuildResult {
    target: String,
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

    to_build: Vec<String>,
    has_changed_files: HashSet<String>,
    built_targets: HashSet<String>,
    building: HashSet<String>,
}

impl Builder {
    fn new(targets: HashMap<String, Target>) -> Builder {
        Builder {
            targets,
            to_build: vec![],
            has_changed_files: Default::default(),
            built_targets: Default::default(),
            building: Default::default(),
        }
    }

    fn sanity_check(&self) {
        for (target_name, target) in self.targets.iter() {
            for dependency in target.depends_on.iter() {
                if !self.targets.contains_key(dependency.as_str()) {
                    panic!("Dependency {} not found.", dependency);
                }
                if target_name == dependency {
                    panic!("Target {} cannot depend on itself.", target_name);
                }
            }
        }
    }

    fn choose_build_targets(&mut self) {
        for (target_name, target) in self.targets.iter() {
            let dependencies_satisfied = target
                .depends_on
                .iter()
                .all(|dependency| self.built_targets.contains(dependency));

            if dependencies_satisfied
                && !self.building.contains(target_name)
                && (!self.built_targets.contains(target_name)
                    || self.has_changed_files.contains(target_name))
            {
                self.to_build.push(target_name.clone());
            }
        }
    }

    fn build_loop(&mut self) {
        /* Choose build targets (based on what's already been built, dependency tree, etc)
        Build all of them in parallel
        Wait for things to be built
        As things get built, check to see if there's something new we can build
        If so, start building that in parallel too

        Stop when nothing is still building and there's nothing left to build */
        crossbeam::scope(|scope| {
            let (_watcher, watcher_rx) = self.setup_watcher();

            let (tx, rx) = unbounded();
            let working_dir = current_dir().unwrap();
            let working_dir = working_dir.to_str().unwrap();

            let mut run_tx_channels: HashMap<String, Sender<RunSignal>> = Default::default();

            loop {
                match watcher_rx.try_recv() {
                    Ok(result) => {
                        let absolute_path = result.path.unwrap();
                        let absolute_path = absolute_path.to_str().unwrap();

                        // TODO: This won't work with symlinks.
                        let relative_path = &absolute_path[working_dir.len() + 1..];

                        for target in self.targets.values() {
                            if target
                                .watch_list
                                .iter()
                                .any(|watch_path| relative_path.starts_with(watch_path))
                            {
                                self.has_changed_files.insert(target.name.clone());
                            }
                        }
                    }
                    Err(e) => {
                        if e != TryRecvError::Empty {
                            panic!("{}", e);
                        }
                    }
                }

                self.choose_build_targets();

                // if self.to_build.len() == 0 && self.building.len() == 0 {
                //    TODO: Exit if nothing to watch.
                //    break;
                // }

                let to_build_clone = self.to_build.clone();
                for target_to_build in to_build_clone {
                    println!("Building {}", target_to_build);
                    self.building.insert(target_to_build.clone());
                    self.has_changed_files.remove(&target_to_build);
                    let tx_clone = tx.clone();
                    let target = self.targets.get(&target_to_build).unwrap().clone();
                    scope.spawn(move |_| target.build(tx_clone));
                }
                self.to_build.clear();

                match rx.try_recv() {
                    Ok(result) => {
                        self.parse_build_result(&result);

                        let target = self.targets.get(&result.target).unwrap().clone();

                        // If already running, send a kill signal.
                        match run_tx_channels.get(&target.name) {
                            None => {}
                            Some(run_tx) => run_tx.send(RunSignal::Kill).unwrap(),
                        }

                        let (run_tx, run_rx) = unbounded();
                        run_tx_channels.insert(target.name.clone(), run_tx);

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
        })
        .unwrap();
    }

    fn setup_watcher(&self) -> (RecommendedWatcher, Receiver<RawEvent>) {
        let (watcher_tx, watcher_rx) = unbounded();
        let mut watcher: RecommendedWatcher = Watcher::new_immediate(watcher_tx).unwrap();
        for target in self.targets.values() {
            for watch_path in target.watch_list.iter() {
                watcher.watch(watch_path, RecursiveMode::Recursive).unwrap();
            }
        }

        (watcher, watcher_rx)
    }

    fn parse_build_result(&mut self, result: &BuildResult) -> () {
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
        self.building.retain(|x| *x != result.target);
        self.built_targets.insert(result.target.clone());
    }
}

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone)]
struct Target {
    #[serde(skip)]
    name: String,
    #[serde(default)]
    depends_on: Vec<String>,
    #[serde(default, rename = "watch")]
    watch_list: Vec<String>,
    #[serde(default, rename = "build")]
    build_list: Vec<String>,
    #[serde(default, rename = "run")]
    run_list: Vec<String>,
}

impl Target {
    fn build(&self, tx: Sender<BuildResult>) {
        let mut output_string = String::from("");

        let mut hasher = Sha1::new();

        if !self.watch_list.is_empty() {
            for path in self.watch_list.iter() {
                let checksum = calculate_checksum(path);
                hasher.input_str(&checksum);
            }

            let watch_checksum = hasher.result_str();
            if does_checksum_match(&self.name, &watch_checksum) {
                tx.send(BuildResult {
                    target: self.name.clone(),
                    state: BuildResultState::Skip,
                    output: output_string,
                })
                .unwrap();
                return;
            }
            write_checksum(&self.name, &watch_checksum);
        }

        for command in self.build_list.iter() {
            println!("Running build command: {}", command);
            match cmd!("/bin/sh", "-c", command).stderr_to_stdout().run() {
                Ok(output) => {
                    println!("Ok {}", command);
                    output_string.push_str(String::from_utf8(output.stdout).unwrap().as_str());
                }
                Err(e) => {
                    println!("Err {} {}", e, command);
                    tx.send(BuildResult {
                        target: self.name.clone(),
                        state: BuildResultState::Fail,
                        output: output_string,
                    })
                    .unwrap();
                    return;
                }
            }
        }

        tx.send(BuildResult {
            target: self.name.clone(),
            state: BuildResultState::Success,
            output: output_string,
        })
        .unwrap();
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
                                    Ok(_) => {},
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

fn calculate_checksum(path: &String) -> String {
    let mut hasher = Sha1::new();

    for entry in WalkDir::new(path) {
        let entry = entry.unwrap();

        if entry.path().is_file() {
            let contents = fs::read(entry.path().to_str().unwrap()).unwrap();
            hasher.input(contents.as_slice());
        }
    }

    return hasher.result_str();
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

fn write_checksum(target: &String, checksum: &String) {
    let file_name = checksum_file_name(target);
    let mut file = fs::File::create(&file_name).expect(&format!(
        "Failed to create checksum file {} for target {}",
        file_name, target
    ));
    file.write_all(checksum.as_bytes()).expect(&format!(
        "Failed to write checksum file {} for target {}",
        file_name, target
    ));
}
