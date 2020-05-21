use anyhow::{Context, Error, Result};
use lazy_static::lazy_static;
use regex::Regex;
use serde::Deserialize;
use std::collections::HashMap;
use std::fs::File;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

#[derive(Debug, Deserialize)]
pub struct Target {
    #[serde(default)]
    pub dependencies: Vec<String>,
    #[serde(default)]
    pub input_paths: Vec<String>,
    #[serde(default)]
    pub output_paths: Vec<String>,
    #[serde(default)]
    pub build: Option<String>,
    #[serde(default)]
    pub service: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct Project {
    #[serde(default)]
    pub imports: Vec<String>,
    #[serde(default)]
    pub targets: HashMap<String, Target>,
}

pub struct Projects(pub HashMap<PathBuf, Project>);

impl Projects {
    pub fn load(root_project_dir: &Path) -> Result<Self> {
        let mut projects = HashMap::new();

        fn add_project(project_dir: &Path, projects: &mut HashMap<PathBuf, Project>) -> Result<()> {
            let project_dir = project_dir.canonicalize().map_err(|e| {
                let context = if e.kind() == ErrorKind::NotFound {
                    format!("Directory {} does not exist", project_dir.display())
                } else {
                    format!("Invalid directory: {}", project_dir.display())
                };
                Error::new(e).context(context)
            })?;

            if projects.contains_key(&project_dir) {
                return Ok(());
            }

            let project = Projects::load_project(&project_dir)?;
            let import_paths: Vec<_> = project
                .imports
                .iter()
                .map(|import| project_dir.join(import))
                .collect();
            projects.insert(project_dir, project);

            for import_path in import_paths {
                add_project(&import_path, projects)?;
            }

            Ok(())
        }

        add_project(root_project_dir, &mut projects)?;

        Ok(Self(projects))
    }

    fn load_project(project_dir: &Path) -> Result<Project> {
        let config_file_path = project_dir.join("zinoma.yml");
        let config_file = File::open(&config_file_path).with_context(|| {
            format!("Failed to open config file {}", config_file_path.display())
        })?;
        let project: Project = serde_yaml::from_reader(config_file)
            .with_context(|| format!("Invalid format for {}", config_file_path.display()))?;

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

    pub fn get_project_dirs(&self) -> Vec<PathBuf> {
        self.0.keys().cloned().collect()
    }
}

pub fn is_valid_target_name(target_name: &str) -> bool {
    lazy_static! {
        static ref RE: Regex = Regex::new(r"^\w[-\w]*$").unwrap();
    }
    RE.is_match(target_name)
}

#[cfg(test)]
mod tests {
    use super::is_valid_target_name;

    #[test]
    fn test_is_valid_target_name() {
        assert!(
            is_valid_target_name("my-target"),
            "A target name can contain letters and hyphens"
        );
        assert!(
            is_valid_target_name("007"),
            "A target name can contain numbers"
        );
        assert!(
            is_valid_target_name("_hidden_target"),
            "A target name can start with underscore"
        );

        assert!(
            !is_valid_target_name("-"),
            "A target name cannot start with an hyphen"
        );
        assert!(!is_valid_target_name(""), "A target name cannot be empty");
    }
}
