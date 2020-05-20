mod conversion;
mod validation;

use crate::domain;
use anyhow::{Context, Result};
use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use validation::validate_targets;

#[derive(Debug, Deserialize)]
pub struct Target {
    #[serde(default)]
    dependencies: Vec<String>,
    #[serde(default)]
    input_paths: Vec<String>,
    #[serde(default)]
    output_paths: Vec<String>,
    #[serde(default)]
    build: Option<String>,
    #[serde(default)]
    service: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct Project {
    #[serde(default)]
    targets: HashMap<String, Target>,
}

pub struct Config {
    projects: HashMap<PathBuf, Project>,
}

impl Config {
    pub fn load(root_project_dir: PathBuf) -> Result<Self> {
        let config_file = root_project_dir.join("zinoma.yml");
        let contents = fs::read_to_string(&config_file)
            .with_context(|| format!("Something went wrong reading {}", config_file.display()))?;
        let project: Project = serde_yaml::from_str(&contents)
            .with_context(|| format!("Invalid format for {}", config_file.display()))?;

        validate_targets(&project.targets).with_context(|| {
            format!(
                "Invalid configuration found in file {}",
                config_file.display()
            )
        })?;

        let projects = vec![(root_project_dir, project)].into_iter().collect();

        Ok(Self { projects })
    }

    pub fn get_target_names(&self) -> Vec<String> {
        self.projects
            .values()
            .flat_map(|project| project.targets.keys().cloned())
            .collect()
    }

    pub fn into_targets(
        self,
        requested_targets: Option<Vec<String>>,
    ) -> Result<Vec<domain::Target>> {
        conversion::into_targets(self.projects, requested_targets)
    }
}

#[cfg(test)]
mod tests {
    use super::Target;
    use std::collections::HashMap;

    pub fn build_targets(data: Vec<(&str, Target)>) -> HashMap<String, Target> {
        data.into_iter().map(|(k, v)| (k.to_string(), v)).collect()
    }
}
