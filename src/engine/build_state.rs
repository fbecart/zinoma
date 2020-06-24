use super::incremental::IncrementalRunResult;
use crate::domain::{Target, TargetId};
use anyhow::Result;
use crossbeam::thread::ScopedJoinHandle;
use std::collections::HashMap;

pub struct TargetBuildStates<'a: 's, 's> {
    targets: &'a HashMap<TargetId, Target>,
    build_states: HashMap<TargetId, TargetBuildState<'s>>,
}

impl<'a, 's> TargetBuildStates<'a, 's> {
    pub fn new(targets: &'a HashMap<TargetId, Target>) -> Self {
        Self {
            targets,
            build_states: targets
                .keys()
                .cloned()
                .map(|target_id| (target_id, TargetBuildState::new()))
                .collect(),
        }
    }

    pub fn set_build_invalidated(&mut self, target_id: &TargetId) {
        self.build_states
            .get_mut(target_id)
            .unwrap()
            .build_invalidated();
    }

    pub fn set_build_started(
        &mut self,
        target_id: &TargetId,
        build_thread: ScopedJoinHandle<'s, ()>,
    ) {
        self.build_states
            .get_mut(target_id)
            .unwrap()
            .build_started(build_thread);
    }

    pub fn set_build_finished(
        &mut self,
        target_id: &TargetId,
        result: &IncrementalRunResult<Result<()>>,
    ) {
        let target_build_state = self.build_states.get_mut(target_id).unwrap();
        if let IncrementalRunResult::Run(Err(_)) = result {
            target_build_state.build_failed();
        } else {
            target_build_state.build_succeeded();
        }
    }

    pub fn get_ready_to_build_targets(&self) -> Vec<TargetId> {
        self.build_states
            .iter()
            .filter(|(_target_id, build_state)| {
                build_state.to_build && build_state.build_thread.is_none()
            })
            .map(|(target_id, _build_state)| target_id)
            .filter(|&target_id| self.has_all_dependencies_built(target_id))
            .cloned()
            .collect()
    }

    fn has_all_dependencies_built(&self, target_id: &TargetId) -> bool {
        let target = &self.targets[target_id];

        target.dependencies().iter().all(|dependency_id| {
            self.build_states[dependency_id].built && self.has_all_dependencies_built(dependency_id)
        })
    }

    pub fn all_are_built(&self) -> bool {
        self.build_states
            .values()
            .all(|build_state| build_state.built)
    }

    pub fn join_all_build_threads(&mut self) {
        for build_state in self.build_states.values_mut() {
            if build_state.build_thread.is_some() {
                build_state.join_build_thread();
            }
        }
    }
}

struct TargetBuildState<'a> {
    to_build: bool,
    build_thread: Option<ScopedJoinHandle<'a, ()>>,
    built: bool,
}

impl<'a> TargetBuildState<'a> {
    pub fn new() -> Self {
        Self {
            to_build: true,
            build_thread: None,
            built: false,
        }
    }

    pub fn build_invalidated(&mut self) {
        self.to_build = true;
        self.built = false;
    }

    pub fn build_started(&mut self, build_thread: ScopedJoinHandle<'a, ()>) {
        self.to_build = false;
        self.build_thread = Some(build_thread);
        self.built = false;
    }

    pub fn build_succeeded(&mut self) {
        self.join_build_thread();
        self.built = !self.to_build;
    }

    pub fn build_failed(&mut self) {
        self.join_build_thread();
        self.built = false;
    }

    fn join_build_thread(&mut self) {
        let build_thread = std::mem::replace(&mut self.build_thread, None);
        build_thread
            .unwrap()
            .join()
            .unwrap_or_else(|_| log::error!("Failed to join build thread"));
    }
}
