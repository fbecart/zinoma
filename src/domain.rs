use anyhow::{anyhow, Result};
use std::fmt;
use std::path::PathBuf;

pub type TargetId = usize;

#[derive(Debug)]
pub struct Target {
    pub id: TargetId,
    pub name: TargetCanonicalName,
    pub project_dir: PathBuf,
    pub dependencies: Vec<TargetId>,
    pub target_type: TargetType,
}

impl fmt::Display for Target {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.name.fmt(fmt)
    }
}

impl Target {
    pub fn get_input(&self) -> Option<&Resources> {
        match &self.target_type {
            TargetType::BuildStep { input, .. } => Some(&input),
            TargetType::Service { input, .. } => Some(&input),
            _ => None,
        }
    }

    pub fn get_output(&self) -> Option<&Resources> {
        match &self.target_type {
            TargetType::BuildStep { output, .. } => Some(&output),
            _ => None,
        }
    }
}

#[derive(Debug)]
pub enum TargetType {
    BuildStep {
        build_script: String,
        input: Resources,
        output: Resources,
    },
    Service {
        run_script: String,
        input: Resources,
    },
    Aggregate,
}

impl TargetType {
    pub fn extend_input(&mut self, resources: &Resources) -> Result<()> {
        match self {
            TargetType::BuildStep { input, .. } => input.extend(resources),
            TargetType::Service { input, .. } => input.extend(resources),
            TargetType::Aggregate => {
                return Err(anyhow!("Can't extend the input of an aggregate target"))
            }
        }

        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TargetCanonicalName {
    pub project_name: Option<String>,
    pub target_name: String,
}

impl TargetCanonicalName {
    pub fn try_parse(target_name: &str, current_project: &Option<String>) -> Result<Self> {
        let parts = target_name.split("::").collect::<Vec<_>>();
        match parts[..] {
            [project_name, target_name] => Ok(Self {
                project_name: Some(project_name.to_owned()),
                target_name: target_name.to_owned(),
            }),
            [target_name] => Ok(Self {
                project_name: current_project.clone(),
                target_name: target_name.to_owned(),
            }),
            _ => Err(anyhow!(
                "Invalid target canonical name: {} (expected a maximum of one '::' delimiter)",
                target_name
            )),
        }
    }

    pub fn try_parse_many(
        target_names: &[String],
        current_project: &Option<String>,
    ) -> Result<Vec<Self>> {
        target_names
            .iter()
            .map(|target_name| Self::try_parse(target_name, current_project))
            .collect()
    }
}

impl fmt::Display for TargetCanonicalName {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(project_name) = &self.project_name {
            fmt.write_fmt(format_args!("{}::", project_name))?;
        }
        fmt.write_str(&self.target_name)
    }
}

#[derive(Debug, PartialEq)]
pub struct Resources {
    pub paths: Vec<PathBuf>,
    pub cmds: Vec<(String, PathBuf)>,
}

impl Resources {
    pub fn new() -> Self {
        Self {
            paths: vec![],
            cmds: vec![],
        }
    }

    pub fn is_empty(&self) -> bool {
        self.paths.is_empty() && self.cmds.is_empty()
    }

    pub fn extend(&mut self, other: &Resources) {
        self.paths.extend_from_slice(&other.paths);
        self.cmds.extend_from_slice(&other.cmds);
    }
}
