use anyhow::{anyhow, Result};
use async_std::path::PathBuf;
use std::collections::{BTreeSet,HashMap};
use std::fmt;

#[derive(Debug, Clone)]
pub struct TargetMetadata {
    pub id: TargetId,
    pub project_dir: PathBuf,
    pub dependencies: Vec<TargetId>,
}

impl fmt::Display for TargetMetadata {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(fmt, "{}", self.id)
    }
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
    pub command: Command,
    pub input: Resources,
}

impl fmt::Display for ServiceTarget {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(fmt, "{}", self.metadata.id)
    }
}

#[derive(Debug)]
pub struct Command {
    pub program: String,
    pub args: Vec<String>,
    pub dir: PathBuf,
    pub env: HashMap<String, String>,
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
    pub fn metadata(&self) -> &TargetMetadata {
        match self {
            Target::Build(target) => &target.metadata,
            Target::Service(target) => &target.metadata,
            Target::Aggregate(target) => &target.metadata,
        }
    }

    pub fn id(&self) -> &TargetId {
        &self.metadata().id
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
            Target::Build(target) => target.input.extend(resources),
            Target::Service(target) => target.input.extend(resources),
            Target::Aggregate(_) => {
                return Err(anyhow!("Can't extend the input of an aggregate target"))
            }
        }

        Ok(())
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

#[derive(Debug, PartialEq, Clone)]
pub struct FilesResource {
    pub paths: Vec<PathBuf>,
    pub extensions: FileExtensions,
}

pub type FileExtensions = Option<BTreeSet<String>>;

pub fn matches_extensions(file: &std::path::Path, extensions: &FileExtensions) -> bool {
    extensions.as_ref().map_or(true, |extensions| {
        let file_name = file.file_name().unwrap().to_string_lossy();
        extensions.iter().any(|ext| file_name.ends_with(ext))
    })
}

#[derive(Debug, PartialEq, Clone)]
pub struct CmdResource {
    pub cmd: String,
    pub dir: PathBuf,
}

#[derive(Debug, PartialEq, Clone)]
pub struct Resources {
    pub files: Vec<FilesResource>,
    pub cmds: Vec<CmdResource>,
}

impl Resources {
    pub fn new() -> Self {
        Self {
            files: vec![],
            cmds: vec![],
        }
    }

    pub fn is_empty(&self) -> bool {
        self.files.is_empty() && self.cmds.is_empty()
    }

    pub fn extend(&mut self, other: &Resources) {
        self.files.extend_from_slice(&other.files);
        self.cmds.extend_from_slice(&other.cmds);
    }
}
