mod conversion;
mod validation;

use crate::domain;
use anyhow::{Context, Result};
use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use validation::{is_valid_target_name, validate_targets_dependency_graph};

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
    import: Vec<String>,
    targets: HashMap<String, Target>,
}

pub struct Config {
    targets: HashMap<String, (PathBuf, Target)>,
}

impl Config {
    pub fn load(root_project_dir: PathBuf) -> Result<Self> {
        let config_file = root_project_dir.join("zinoma.yml");
        let project = Self::load_project(&root_project_dir)?;

        let projects: HashMap<PathBuf, Project> = vec![(root_project_dir.canonicalize()?, project)]
            .into_iter()
            .collect();

        let targets: HashMap<String, (PathBuf, Target)> = projects
            .into_iter()
            .flat_map(|(project_dir, project)| {
                project
                    .targets
                    .into_iter()
                    .map(|(target_name, target)| (target_name, (project_dir.clone(), target)))
                    .collect::<Vec<_>>()
            })
            .collect();

        validate_targets_dependency_graph(&targets).with_context(|| {
            format!(
                "Invalid configuration found in file {}",
                config_file.display()
            )
        })?;

        Ok(Self { targets })
    }

    fn load_project(project_dir: &Path) -> Result<Project> {
        let config_file = project_dir.join("zinoma.yml");
        let contents = fs::read_to_string(&config_file)
            .with_context(|| format!("Something went wrong reading {}", config_file.display()))?;
        let project: Project = serde_yaml::from_str(&contents)
            .with_context(|| format!("Invalid format for {}", config_file.display()))?;

        if let Some(invalid_target_name) = project
            .targets
            .keys()
            .find(|&target_name| !is_valid_target_name(target_name))
        {
            return Err(anyhow::anyhow!(
                "{} is not a valid target name",
                invalid_target_name
            ));
        }

        Ok(project)
    }

    pub fn get_target_names(&self) -> Vec<String> {
        self.targets.keys().cloned().collect()
    }

    pub fn into_targets(
        self,
        requested_targets: Option<Vec<String>>,
    ) -> Result<Vec<domain::Target>> {
        conversion::into_targets(self.targets, requested_targets)
    }
}

#[cfg(test)]
mod tests {
    use super::Target;
    use std::collections::HashMap;
    use std::path::PathBuf;

    pub fn build_targets(data: Vec<(&str, Target)>) -> HashMap<String, (PathBuf, Target)> {
        data.into_iter()
            .map(|(k, v)| (k.to_string(), (PathBuf::from("."), v)))
            .collect()
    }
}
