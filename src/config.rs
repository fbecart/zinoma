use crate::target;
use serde::Deserialize;
use std::collections::HashMap;
use std::fmt;
use std::fs;
use std::path::Path;

#[derive(Debug, Deserialize)]
struct Target {
    #[serde(default)]
    depends_on: Vec<String>,
    #[serde(default, rename = "watch")]
    watch_list: Vec<String>,
    #[serde(default, rename = "build")]
    build_list: Vec<String>,
    #[serde(default)]
    run: Option<String>,
    #[serde(default)]
    run_options: RunOptions,
}

#[derive(Debug, Deserialize)]
struct RunOptions {
    #[serde(default)]
    incremental: bool,
}

impl Default for RunOptions {
    fn default() -> Self {
        Self { incremental: true }
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

pub struct Config {
    targets: HashMap<String, Target>,
}

impl Config {
    pub fn from_yml_file(file: &Path) -> Result<Self, String> {
        let contents = fs::read_to_string(file)
            .map_err(|e| format!("Something went wrong reading {}: {}", file.display(), e))?;
        let targets = serde_yaml::from_str(&contents)
            .map_err(|e| format!("Invalid format for {}: {}", file.display(), e))?;
        Self::check_targets(&targets).map_err(|e| format!("Failed sanity check: {}", e))?;

        Ok(Self { targets })
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

    pub fn into_targets(self, requested_targets: &[String]) -> Result<Vec<target::Target>, String> {
        self.validate_requested_targets(requested_targets)?;

        let Self {
            targets: mut raw_targets,
        } = self;

        let mut targets = Vec::with_capacity(requested_targets.len());
        let mut mapping = HashMap::with_capacity(requested_targets.len());

        fn add_target(
            mut targets: &mut Vec<target::Target>,
            mut mapping: &mut HashMap<String, target::TargetId>,
            raw_targets: &mut HashMap<String, Target>,
            target_name: &str,
        ) {
            if mapping.contains_key(target_name) {
                return;
            }

            let Target {
                depends_on,
                watch_list,
                build_list,
                run,
                run_options,
            } = raw_targets.remove(target_name).unwrap();
            depends_on.iter().for_each(|dependency| {
                add_target(&mut targets, &mut mapping, raw_targets, dependency)
            });

            let target_id = targets.len();
            mapping.insert(target_name.to_string(), target_id);
            let depends_on = depends_on
                .iter()
                .map(|target_name| *mapping.get(target_name).unwrap())
                .collect();
            let watch_list = watch_list
                .iter()
                .map(|watch| Path::new(watch).to_path_buf())
                .collect();
            targets.push(target::Target::new(
                target_id,
                target_name.to_string(),
                depends_on,
                watch_list,
                build_list,
                run,
                run_options.incremental,
            ));
        }

        requested_targets.iter().for_each(|requested_target| {
            add_target(
                &mut targets,
                &mut mapping,
                &mut raw_targets,
                requested_target,
            )
        });

        Ok(targets)
    }

    fn validate_requested_targets(&self, requested_targets: &[String]) -> Result<(), String> {
        let invalid_targets: Vec<String> = requested_targets
            .iter()
            .filter(|requested_target| !self.targets.contains_key(*requested_target))
            .map(|i| i.to_owned())
            .collect();

        if invalid_targets.is_empty() {
            Ok(())
        } else {
            Err(format!("Invalid targets: {}", invalid_targets.join(", ")))
        }
    }
}
