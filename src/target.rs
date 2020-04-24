pub type TargetId = usize;

#[derive(Clone, Debug)]
pub struct Target {
    pub id: TargetId,
    pub name: String,
    pub depends_on: Vec<TargetId>,
    pub watch_list: Vec<String>,
    pub build_list: Vec<String>,
    pub run: Option<String>,
    pub incremental_run: bool,
}

impl Target {
    pub fn new(
        id: TargetId,
        name: String,
        depends_on: Vec<TargetId>,
        watch_list: Vec<String>,
        build_list: Vec<String>,
        run: Option<String>,
        incremental_run: bool,
    ) -> Self {
        Self {
            id,
            name,
            depends_on,
            watch_list,
            build_list,
            run,
            incremental_run,
        }
    }
}
