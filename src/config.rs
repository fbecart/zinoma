use crate::target;
use anyhow::{Context, Result};
use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

#[derive(Debug, Deserialize)]
struct Target {
    #[serde(default)]
    dependencies: Vec<String>,
    #[serde(default)]
    input_paths: Vec<String>,
    #[serde(default)]
    output_paths: Vec<String>,
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
        let targets: HashMap<String, Target> = serde_yaml::from_str(&contents)
            .with_context(|| format!("Invalid format for {}", file.display()))?;

        for target_name in targets.keys() {
            Self::validate_target(target_name, &[], &targets)
                .with_context(|| format!("Invalid configuration in file {}", file.display()))?;
        }

        Ok(Self { targets })
    }

    /// Checks the validity of the provided target.
    ///
    /// Ensures that all target dependencies (both direct and transitive) exist,
    /// and that the dependency graph has no circular dependency.
    fn validate_target(
        target_name: &str,
        parent_targets: &[&str],
        targets: &HashMap<String, Target>,
    ) -> Result<()> {
        let target = targets
            .get(target_name)
            .ok_or_else(|| anyhow::anyhow!("Target {} not found", target_name))?;

        if parent_targets.contains(&target_name) {
            return Err(anyhow::anyhow!(
                "Circular dependency: {} -> {}",
                parent_targets.join(" -> "),
                target_name
            ));
        }

        let targets_chain = [parent_targets, &[target_name]].concat();
        for dependency in target.dependencies.iter() {
            Self::validate_target(dependency, &targets_chain, &targets)?;
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
                dependencies,
                input_paths,
                output_paths,
                build_list,
                service,
            } = raw_targets.remove(target_name).unwrap();
            dependencies.iter().for_each(|dependency| {
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
            let dependencies = dependencies
                .iter()
                .map(|target_name| *mapping.get(target_name).unwrap())
                .collect();
            let input_paths = input_paths
                .iter()
                .map(|path| project_dir.join(path))
                .collect();
            let output_paths = output_paths
                .iter()
                .map(|path| project_dir.join(path))
                .collect();
            targets.push(target::Target {
                id: target_id,
                name: target_name.to_string(),
                dependencies,
                path: project_dir.to_path_buf(),
                input_paths,
                output_paths,
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
            return Err(anyhow::anyhow!(
                "Invalid targets: {}",
                invalid_targets.join(", ")
            ));
        }

        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::{Config, Target};
    use anyhow::Result;
    use std::collections::HashMap;

    #[test]
    fn test_validate_targets_on_valid_targets() -> Result<()> {
        let targets = build_targets(vec![
            ("target_1", build_target_with_dependencies(vec!["target_2"])),
            ("target_2", build_target_with_dependencies(vec![])),
        ]);

        Config::validate_target("target_1", &[], &targets)
    }

    #[test]
    fn test_validate_targets_with_unknown_dependency() {
        let targets = build_targets(vec![(
            "target_1",
            build_target_with_dependencies(vec!["target_2"]),
        )]);

        let result = Config::validate_target("target_1", &[], &targets);

        assert_eq!("Target target_2 not found", result.unwrap_err().to_string());
    }

    #[test]
    fn test_validate_targets_with_circular_dependency() {
        let targets = build_targets(vec![
            ("target_1", build_target_with_dependencies(vec!["target_2"])),
            ("target_2", build_target_with_dependencies(vec!["target_3"])),
            ("target_3", build_target_with_dependencies(vec!["target_1"])),
        ]);

        let result = Config::validate_target("target_1", &[], &targets);

        assert_eq!(
            "Circular dependency: target_1 -> target_2 -> target_3 -> target_1",
            result.unwrap_err().to_string()
        );
    }

    fn build_target_with_dependencies(dependencies: Vec<&str>) -> Target {
        Target {
            dependencies: dependencies.iter().map(|&dep| dep.to_string()).collect(),
            input_paths: vec![],
            output_paths: vec![],
            build_list: vec![],
            service: None,
        }
    }

    fn build_targets(data: Vec<(&str, Target)>) -> HashMap<String, Target> {
        data.into_iter().map(|(k, v)| (k.to_string(), v)).collect()
    }
}
