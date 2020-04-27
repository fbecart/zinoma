use std::path::PathBuf;

pub type TargetId = usize;

#[derive(Clone, Debug)]
pub struct Target {
    pub id: TargetId,
    pub name: String,
    pub depends_on: Vec<TargetId>,
    pub path: PathBuf,
    pub watch_list: Vec<PathBuf>,
    pub build_list: Vec<String>,
    pub service: Option<String>,
}

impl Target {
    pub fn new(
        id: TargetId,
        name: String,
        depends_on: Vec<TargetId>,
        path: PathBuf,
        watch_list: Vec<PathBuf>,
        build_list: Vec<String>,
        service: Option<String>,
    ) -> Self {
        Self {
            id,
            name,
            depends_on,
            path,
            watch_list,
            build_list,
            service,
        }
    }
}
