mod schema;

use anyhow::{Context, Error, Result};
use lazy_static::lazy_static;
use regex::Regex;
pub use schema::{Project, Target};
use std::collections::HashMap;
use std::fs::File;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

pub struct Config {
    pub root_project_dir: PathBuf,
    pub projects: HashMap<PathBuf, Project>,
}

impl Config {
    pub fn load(root_project_dir: &Path) -> Result<Self> {
        let root_project_dir = canonicalize_dir(root_project_dir)?;

        let mut projects = HashMap::new();

        fn add_project(
            project_dir: PathBuf,
            projects: &mut HashMap<PathBuf, Project>,
        ) -> Result<()> {
            if projects.contains_key(&project_dir) {
                return Ok(());
            }

            let project = Config::load_project(&project_dir)?;

            let import_paths = project
                .imports
                .iter()
                .map(|(import_name, import_dir)| {
                    canonicalize_dir(&project_dir.join(import_dir))
                        .map(|dir| (import_name.clone(), dir))
                })
                .collect::<Result<Vec<_>>>()?;
            projects.insert(project_dir, project);

            for (import_name, import_dir) in import_paths {
                add_project(import_dir.clone(), projects)
                    .and_then(|_| match &projects[&import_dir].name {
                        None => Err(anyhow::anyhow!(
                            "Project cannot be imported as it has no name"
                        )),
                        Some(name) if name != &import_name => Err(anyhow::anyhow!(
                            "The project should be imported with name {}",
                            name
                        )),
                        _ => Ok(()),
                    })
                    .with_context(|| format!("Failed to import {}", &import_name))?;
            }

            Ok(())
        }

        add_project(root_project_dir.clone(), &mut projects)?;

        Ok(Self {
            root_project_dir,
            projects,
        })
    }

    fn load_project(project_dir: &Path) -> Result<Project> {
        let config_file_path = project_dir.join("zinoma.yml");
        let config_file = File::open(&config_file_path).with_context(|| {
            format!("Failed to open config file {}", config_file_path.display())
        })?;
        let project: Project = serde_yaml::from_reader(config_file)
            .with_context(|| format!("Invalid format for {}", config_file_path.display()))?;

        if let Some(project_name) = &project.name {
            if !is_valid_project_name(&project_name) {
                return Err(anyhow::anyhow!(
                    "{} is not a valid project name",
                    project_name
                ));
            }
        }

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
        self.projects.keys().cloned().collect()
    }
}

fn canonicalize_dir(dir: &Path) -> Result<PathBuf> {
    dir.canonicalize().map_err(|e| {
        let context = if e.kind() == ErrorKind::NotFound {
            format!("Directory {} does not exist", dir.display())
        } else {
            format!("Invalid directory: {}", dir.display())
        };
        Error::new(e).context(context)
    })
}

pub fn is_valid_target_name(target_name: &str) -> bool {
    lazy_static! {
        static ref RE: Regex = Regex::new(r"^\w[-\w]*$").unwrap();
    }
    RE.is_match(target_name)
}

pub fn is_valid_project_name(project_name: &str) -> bool {
    lazy_static! {
        static ref RE: Regex = Regex::new(r"^\w[-\w]*$").unwrap();
    }
    RE.is_match(project_name)
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
