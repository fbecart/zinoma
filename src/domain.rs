use std::fmt;
use std::path::PathBuf;

pub type TargetId = usize;

#[derive(Clone, Debug)]
pub struct Target {
    pub id: TargetId,
    pub name: String,
    pub project: Project,
    pub dependencies: Vec<TargetId>,
    pub input: Resources,
    pub output: Resources,
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
pub struct Resources {
    pub paths: Vec<PathBuf>,
    pub cmds: Vec<String>,
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
}
