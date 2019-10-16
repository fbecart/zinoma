use crossbeam;
use crypto::digest::Digest;
use crypto::sha1::Sha1;
use duct::cmd;
use serde::{Serialize, Deserialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::Write;
use std::sync::mpsc;
use walkdir::WalkDir;

fn main() {
    let file_name = ".buildy.yml";
    let contents = fs::read_to_string(file_name)
        .expect(&format!("Something went wrong reading {}.", file_name));

    let mut targets: HashMap<String, Target> = serde_yaml::from_str(&contents)
        .expect(&format!("Invalid format for {}.", file_name));
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

struct Builder {
    targets: HashMap<String, Target>,
    to_build: Vec<String>,
    built_targets: HashSet<String>,
    building: HashSet<String>,
}

impl Builder {
    fn new(targets: HashMap<String, Target>) -> Builder {
        Builder{
            targets,
            to_build: vec![],
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
            let mut dependencies_satisfied = true;
            for dependency in target.depends_on.iter() {
                if !self.built_targets.contains(dependency) {
                    dependencies_satisfied = false;
                    break;
                }
            }

            if dependencies_satisfied &&
                !self.building.contains(target_name) &&
                !self.built_targets.contains(target_name) {
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
            let (tx, rx) = mpsc::sync_channel(0);
            loop {
                self.choose_build_targets();
                if self.to_build.len() == 0 && self.building.len() == 0 {
                    println!("Could not build anything else.");
                    break;
                }

                let to_build_clone = self.to_build.clone();
                for target_to_build in to_build_clone {
                    println!("Building {}", target_to_build);
                    self.building.insert(target_to_build.clone());
                    let tx_clone = tx.clone();
                    let target = self.targets.get(&target_to_build).unwrap().clone();
                    scope.spawn(move |_| target.build(tx_clone));
                }
                self.to_build.clear();

                match rx.recv() {
                    Ok(result) => {
                        self.parse_build_result(&result);
                        let tx_clone = tx.clone();
                        let target = self.targets.get(&result.target).unwrap().clone();
                        scope.spawn(move |_| target.run(tx_clone));
                    }
                    Err(e) => {
                        println!("{:?}", e);
                    }
                }
            }
        }).unwrap();
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
    fn build(&self, tx: mpsc::SyncSender<BuildResult>) {
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
                }).unwrap();
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
                    }).unwrap();
                    return;
                }
            }
        }

        tx.send(BuildResult {
            target: self.name.clone(),
            state: BuildResultState::Success,
            output: output_string,
        }).unwrap();
    }

    fn run(&self, _tx: mpsc::SyncSender<BuildResult>) {
        for command in self.run_list.iter() {
            println!("Running command: {}", command);
            match cmd!("/bin/sh", "-c", command).stderr_to_stdout().run() {
                Ok(_) => {
                    println!("Ok {}", command);
                }
                Err(e) => {
                    println!("Err {} {}", e, command);
                    return;
                }
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
            panic!("Failed reading checksum file {} for target {}: {}", file_name, target, e);
        }
    };
}

fn write_checksum(target: &String, checksum: &String) {
    let file_name = checksum_file_name(target);
    let mut file = fs::File::create(&file_name)
        .expect(&format!("Failed to create checksum file {} for target {}", file_name, target));
    file.write_all(checksum.as_bytes())
        .expect(&format!("Failed to write checksum file {} for target {}", file_name, target));
}
