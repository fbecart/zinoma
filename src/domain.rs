use anyhow::{anyhow, Result};
use std::fmt;
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub struct TargetMetadata {
    pub id: TargetId,
    pub project_dir: PathBuf,
    pub dependencies: Vec<TargetId>,
}

#[derive(Debug)]
pub struct BuildTarget {
    pub metadata: TargetMetadata,
    pub build_script: String,
    pub input: Resources,
    pub output: Resources,
}

impl fmt::Display for BuildTarget {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(fmt, "{}", self.metadata.id)
    }
}

#[derive(Debug)]
pub struct ServiceTarget {
    pub metadata: TargetMetadata,
    pub run_script: String,
    pub input: Resources,
}

impl fmt::Display for ServiceTarget {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(fmt, "{}", self.metadata.id)
    }
}

#[derive(Debug)]
pub struct AggregateTarget {
    pub metadata: TargetMetadata,
}

#[derive(Debug)]
pub enum Target {
    Build(BuildTarget),
    Service(ServiceTarget),
    Aggregate(AggregateTarget),
}

impl Target {
    fn metadata(&self) -> &TargetMetadata {
        match self {
            Target::Build(target) => &target.metadata,
            Target::Service(target) => &target.metadata,
            Target::Aggregate(target) => &target.metadata,
        }
    }

    pub fn id(&self) -> &TargetId {
        &self.metadata().id
    }

    pub fn project_dir(&self) -> &Path {
        &self.metadata().project_dir
    }

    pub fn dependencies(&self) -> &Vec<TargetId> {
        &self.metadata().dependencies
    }

    pub fn extend_dependencies(&mut self, additional_dependencies: &[TargetId]) {
        let metadata = match self {
            Target::Build(target) => &mut target.metadata,
            Target::Service(target) => &mut target.metadata,
            Target::Aggregate(target) => &mut target.metadata,
        };
        metadata
            .dependencies
            .extend_from_slice(additional_dependencies);
    }

    pub fn input(&self) -> Option<&Resources> {
        match self {
            Target::Build(target) => Some(&target.input),
            Target::Service(target) => Some(&target.input),
            _ => None,
        }
    }

    pub fn output(&self) -> Option<&Resources> {
        match self {
            Target::Build(target) => Some(&target.output),
            _ => None,
        }
    }

    pub fn extend_input(&mut self, resources: &Resources) -> Result<()> {
        match self {
            Target::Build(target) => Ok(target.input.extend(resources)),
            Target::Service(target) => Ok(target.input.extend(resources)),
            Target::Aggregate(_) => Err(anyhow!("Can't extend the input of an aggregate target")),
        }
    }
}

impl fmt::Display for Target {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(fmt, "{}", self.id())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TargetId {
    pub project_name: Option<String>,
    pub target_name: String,
}

impl TargetId {
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

impl fmt::Display for TargetId {
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
