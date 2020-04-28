use crate::target::{Target, TargetId};

pub struct TargetBuildStates<'a> {
    targets: &'a [Target],
    build_states: Vec<TargetBuildState>,
}

impl<'a> TargetBuildStates<'a> {
    pub fn new(targets: &'a [Target]) -> Self {
        Self {
            targets,
            build_states: vec![TargetBuildState::new(); targets.len()],
        }
    }

    pub fn set_build_invalidated(&mut self, target_id: TargetId) {
        self.build_states[target_id].build_invalidated();
    }

    pub fn set_build_started(&mut self, target_id: TargetId) {
        self.build_states[target_id].build_started();
    }

    pub fn set_build_succeeded(&mut self, target_id: TargetId) {
        self.build_states[target_id].build_succeeded();
    }

    pub fn set_build_failed(&mut self, target_id: TargetId) {
        self.build_states[target_id].build_failed();
    }

    pub fn get_ready_to_build_targets(&self) -> Vec<TargetId> {
        self.build_states
            .iter()
            .enumerate()
            .filter(|(_target_id, build_state)| build_state.to_build && !build_state.being_built)
            .map(|(target_id, _build_state)| target_id)
            .filter(|&target_id| self.has_all_dependencies_built(target_id))
            .collect()
    }

    fn has_all_dependencies_built(&self, target_id: TargetId) -> bool {
        let target = &self.targets[target_id];

        target.depends_on.iter().all(|&dependency_id| {
            self.build_states[dependency_id].built && self.has_all_dependencies_built(dependency_id)
        })
    }

    pub fn are_all_built(&self) -> bool {
        self.build_states
            .iter()
            .all(|build_state| build_state.built)
    }
}

#[derive(Clone)]
struct TargetBuildState {
    to_build: bool,
    being_built: bool,
    built: bool,
}

impl TargetBuildState {
    pub fn new() -> Self {
        Self {
            to_build: true,
            being_built: false,
            built: false,
        }
    }

    pub fn build_invalidated(&mut self) {
        self.to_build = true;
        self.built = false;
    }

    pub fn build_started(&mut self) {
        self.to_build = false;
        self.being_built = true;
        self.built = false;
    }

    pub fn build_succeeded(&mut self) {
        self.being_built = false;
        self.built = !self.to_build;
    }

    pub fn build_failed(&mut self) {
        self.being_built = false;
        self.built = false;
    }
}
