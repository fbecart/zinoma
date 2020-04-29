use crate::target;
use anyhow::{anyhow, Context, Result};
use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

#[derive(Debug, Deserialize)]
struct Target {
    #[serde(default)]
    depends_on: Vec<String>,
    #[serde(default)]
    input_paths: Vec<String>,
    #[serde(default, rename = "build")]
    build_list: Vec<String>,
    #[serde(default)]
    service: Option<String>,
}

pub struct Config {
    targets: HashMap<String, Target>,
}

impl Config {
    pub fn from_yml_file(file: &Path) -> Result<Self> {
        let contents = fs::read_to_string(file)
            .with_context(|| format!("Something went wrong reading {}", file.display()))?;
        let targets = serde_yaml::from_str(&contents)
            .with_context(|| format!("Invalid format for {}", file.display()))?;
        Self::check_targets(&targets).with_context(|| "Failed sanity check")?;

        Ok(Self { targets })
    }

    fn check_targets(targets: &HashMap<String, Target>) -> Result<()> {
        for (target_name, target) in targets.iter() {
            for dependency in target.depends_on.iter() {
                if !targets.contains_key(dependency.as_str()) {
                    return Err(anyhow!("Dependency {} not found", dependency));
                }
                if target_name == dependency {
                    return Err(anyhow!("Dependency loop: {}", target_name)); // TODO Check recursively
                }
            }
        }
        Ok(())
    }

    pub fn into_targets(
        self,
        project_dir: &Path,
        requested_targets: &[String],
    ) -> Result<Vec<target::Target>> {
        self.validate_requested_targets(requested_targets)?;

        let Self {
            targets: mut raw_targets,
        } = self;

        let mut targets = Vec::with_capacity(requested_targets.len());
        let mut mapping = HashMap::with_capacity(requested_targets.len());

        fn add_target(
            mut targets: &mut Vec<target::Target>,
            mut mapping: &mut HashMap<String, target::TargetId>,
            project_dir: &Path,
            raw_targets: &mut HashMap<String, Target>,
            target_name: &str,
        ) {
            if mapping.contains_key(target_name) {
                return;
            }

            let Target {
                depends_on,
                input_paths,
                build_list,
                service,
            } = raw_targets.remove(target_name).unwrap();
            depends_on.iter().for_each(|dependency| {
                add_target(
                    &mut targets,
                    &mut mapping,
                    project_dir,
                    raw_targets,
                    dependency,
                )
            });

            let target_id = targets.len();
            mapping.insert(target_name.to_string(), target_id);
            let depends_on = depends_on
                .iter()
                .map(|target_name| *mapping.get(target_name).unwrap())
                .collect();
            let input_paths = input_paths.iter().map(|path| project_dir.join(path)).collect();
            targets.push(target::Target {
                id: target_id,
                name: target_name.to_string(),
                depends_on,
                path: project_dir.to_path_buf(),
                input_paths: input_paths,
                build_list,
                service,
            });
        }

        requested_targets.iter().for_each(|requested_target| {
            add_target(
                &mut targets,
                &mut mapping,
                project_dir,
                &mut raw_targets,
                requested_target,
            )
        });

        Ok(targets)
    }

    fn validate_requested_targets(&self, requested_targets: &[String]) -> Result<()> {
        let invalid_targets: Vec<String> = requested_targets
            .iter()
            .filter(|requested_target| !self.targets.contains_key(*requested_target))
            .map(|i| i.to_owned())
            .collect();

        if !invalid_targets.is_empty() {
            return Err(anyhow!("Invalid targets: {}", invalid_targets.join(", ")));
        }

        Ok(())
    }
}
