use std::fmt;
use std::path::PathBuf;

pub type TargetId = usize;

#[derive(Clone, Debug)]
pub struct Target {
    pub id: TargetId,
    pub name: String,
    pub project: Project,
    pub dependencies: Vec<TargetId>,
    pub inputs: EnvProbes,
    pub outputs: EnvProbes,
    pub build: Option<String>,
    pub service: Option<String>,
}

impl fmt::Display for Target {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(project_name) = &self.project.name {
            fmt.write_fmt(format_args!("{}::", project_name))?;
        }
        fmt.write_str(&self.name)
    }
}

#[derive(Clone, Debug)]
pub struct Project {
    pub dir: PathBuf,
    pub name: Option<String>,
}

#[derive(Clone, Debug)]
pub struct EnvProbes {
    pub paths: Vec<PathBuf>,
    pub cmd_outputs: Vec<String>,
    pub env_vars: Vec<String>,
}

impl EnvProbes {
    pub fn new() -> Self {
        Self {
            paths: vec![],
            cmd_outputs: vec![],
            env_vars: vec![],
        }
    }

    pub fn is_empty(&self) -> bool {
        self.paths.is_empty() && self.cmd_outputs.is_empty() && self.env_vars.is_empty()
    }
}
