pub type TargetId = usize;

#[derive(Clone)]
pub struct Target {
    pub(crate) id: TargetId,
    pub(crate) name: String,
    pub(crate) depends_on: Vec<TargetId>,
    pub(crate) watch_list: Vec<String>,
    pub(crate) build_list: Vec<String>,
    pub(crate) run: Option<String>,
    pub(crate) incremental_run: bool,
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
