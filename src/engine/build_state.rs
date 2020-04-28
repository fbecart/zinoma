use super::builder::BuildReport;
use crate::incremental::IncrementalRunResult;
use crate::target::{Target, TargetId};
use crossbeam::channel::{unbounded, Receiver, Sender, TryRecvError};

pub struct TargetBuildStates<'a> {
    targets: &'a [Target],
    build_states: Vec<TargetBuildState>,
    pub tx: Sender<BuildReport>,
    rx: Receiver<BuildReport>,
}

impl<'a> TargetBuildStates<'a> {
    pub fn new(targets: &'a [Target]) -> Self {
        let (tx, rx) = unbounded();
        Self {
            targets,
            build_states: vec![TargetBuildState::new(); targets.len()],
            tx,
            rx,
        }
    }

    pub fn set_builds_invalidated(&mut self, target_ids: &[TargetId]) {
        for &target_id in target_ids {
            self.build_states[target_id].build_invalidated();
        }
    }

    pub fn set_build_started(&mut self, target_id: TargetId) {
        self.build_states[target_id].build_started();
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

    pub fn all_are_built(&self) -> bool {
        self.build_states
            .iter()
            .all(|build_state| build_state.built)
    }

    pub fn get_finished_build(&mut self) -> Result<Option<BuildReport>, String> {
        match self.rx.try_recv() {
            Ok(result) => {
                let target_build_state = &mut self.build_states[result.target_id];
                if let IncrementalRunResult::Run(Err(_)) = &result.result {
                    target_build_state.build_failed();
                } else {
                    target_build_state.build_succeeded();
                }

                Ok(Some(result))
            }
            Err(TryRecvError::Empty) => Ok(None),
            Err(e) => Err(format!("Crossbeam parallelism failure: {}", e)),
        }
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
