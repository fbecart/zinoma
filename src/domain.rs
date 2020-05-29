use std::fmt;
use std::path::PathBuf;

pub type TargetId = usize;

#[derive(Clone, Debug)]
pub struct Target {
    pub id: TargetId,
    pub name: String,
    pub project: Project,
    pub dependencies: Vec<TargetId>,
    pub input_paths: Vec<PathBuf>,
    pub output_paths: Vec<PathBuf>,
    pub build: Option<String>,
    pub service: Option<String>,
}

impl fmt::Display for Target {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(project_name) = &self.project.name {
            fmt.write_fmt(format_args!("{}::", project_name))?;
        }
        fmt.write_str(&self.name)?;
        Ok(())
    }
}

#[derive(Clone, Debug)]
pub struct Project {
    pub dir: PathBuf,
    pub name: Option<String>,
}
