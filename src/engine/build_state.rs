use super::{incremental::IncrementalRunResult, BuildCancellationMessage};
use crate::domain::{Target, TargetId};
use anyhow::Result;
use async_std::sync::Sender;
use async_std::task::JoinHandle;
use futures::future;
use std::collections::HashMap;

pub struct TargetBuildStates<'a> {
    targets: &'a HashMap<TargetId, Target>,
    build_states: HashMap<TargetId, TargetBuildState>,
}

impl<'a> TargetBuildStates<'a> {
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
        build_thread: JoinHandle<()>,
        build_cancellation_sender: Sender<BuildCancellationMessage>,
    ) {
        self.build_states
            .get_mut(target_id)
            .unwrap()
            .build_started(build_thread, build_cancellation_sender);
    }

    pub async fn set_build_finished(
        &mut self,
        target_id: &TargetId,
        result: &IncrementalRunResult<Result<()>>,
    ) {
        let target_build_state = self.build_states.get_mut(target_id).unwrap();
        if let IncrementalRunResult::Run(Err(_)) = result {
            target_build_state.build_failed().await;
        } else {
            target_build_state.build_succeeded().await;
        }
    }

    pub fn get_ready_to_build_targets(&self) -> Vec<TargetId> {
        self.build_states
            .iter()
            .filter(|(_target_id, build_state)| {
                build_state.to_build && build_state.build_handles.is_none()
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

    pub async fn cancel_all_builds(&mut self) {
        let cancellation_futures = self
            .build_states
            .iter_mut()
            .filter(|(_target_id, build_state)| build_state.build_handles.is_some())
            .map(|(target_id, build_state)| async move {
                log::debug!("{} - Cancelling build", target_id);
                build_state.cancel_build().await;
            });

        future::join_all(cancellation_futures).await;
        log::debug!("All build processes successfully cancelled");
    }
}

struct TargetBuildState {
    to_build: bool,
    build_handles: Option<(JoinHandle<()>, Sender<BuildCancellationMessage>)>,
    built: bool,
}

impl TargetBuildState {
    pub fn new() -> Self {
        Self {
            to_build: true,
            build_handles: None,
            built: false,
        }
    }

    pub fn build_invalidated(&mut self) {
        self.to_build = true;
        self.built = false;
    }

    pub fn build_started(
        &mut self,
        build_thread: JoinHandle<()>,
        build_cancellation_sender: Sender<BuildCancellationMessage>,
    ) {
        self.to_build = false;
        self.build_handles = Some((build_thread, build_cancellation_sender));
        self.built = false;
    }

    pub async fn build_succeeded(&mut self) {
        self.join_build_thread().await;
        self.built = !self.to_build;
    }

    pub async fn build_failed(&mut self) {
        self.join_build_thread().await;
        self.built = false;
    }

    async fn cancel_build(&mut self) {
        let (build_thread, build_cancellation_sender) =
            std::mem::replace(&mut self.build_handles, None).unwrap();
        build_cancellation_sender
            .send(BuildCancellationMessage::CancelBuild)
            .await;
        build_thread.await;
    }

    async fn join_build_thread(&mut self) {
        let (build_thread, _build_cancellation_sender) =
            std::mem::replace(&mut self.build_handles, None).unwrap();
        build_thread.await;
    }
}
