use crate::config::yaml;
use std::fmt;
use std::path::PathBuf;

pub type TargetId = usize;

#[derive(Debug)]
pub struct Target {
    pub id: TargetId,
    pub name: String,
    pub project: Project,
    pub dependencies: Vec<TargetId>,
    raw: yaml::Target,
}

impl fmt::Display for Target {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(project_name) = &self.project.name {
            fmt.write_fmt(format_args!("{}::", project_name))?;
        }
        fmt.write_str(&self.name)
    }
}

impl Target {
    pub fn new(
        id: TargetId,
        name: String,
        project: Project,
        dependencies: Vec<TargetId>,
        raw: yaml::Target,
    ) -> Self {
        Self {
            id,
            name,
            project,
            dependencies,
            raw,
        }
    }

    pub fn input_paths(&self) -> Vec<PathBuf> {
        self.resources_paths(&self.raw.input)
    }

    pub fn input(&self) -> &Vec<yaml::Resource> {
        &self.raw.input
    }

    pub fn output_paths(&self) -> Vec<PathBuf> {
        self.resources_paths(&self.raw.output)
    }

    pub fn output(&self) -> &Vec<yaml::Resource> {
        &self.raw.output
    }

    pub fn build(&self) -> &Option<String> {
        &self.raw.build
    }

    pub fn service(&self) -> &Option<String> {
        &self.raw.service
    }

    fn resources_paths(&self, resources: &[yaml::Resource]) -> Vec<PathBuf> {
        resources
            .iter()
            .filter_map(|resource| {
                if let yaml::Resource::Paths { paths } = resource {
                    Some(
                        paths
                            .iter()
                            .map(|path| self.project.dir.join(path))
                            .collect::<Vec<_>>(),
                    )
                } else {
                    None
                }
            })
            .flatten()
            .collect()
    }
}

#[derive(Debug)]
pub struct Project {
    pub dir: PathBuf,
    pub name: Option<String>,
}
