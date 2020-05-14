use std::path::PathBuf;

pub type TargetId = usize;

#[derive(Clone, Debug)]
pub struct Target {
    pub id: TargetId,
    pub name: String,
    pub dependencies: Vec<TargetId>,
    pub path: PathBuf,
    pub input_paths: Vec<PathBuf>,
    pub output_paths: Vec<PathBuf>,
    pub build: Option<String>,
    pub service: Option<String>,
}
