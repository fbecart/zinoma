use super::{incremental::IncrementalRunResult, BuildCancellationMessage};
use crate::domain::{Target, TargetId};
use anyhow::Result;
use async_std::sync::Sender;
use async_std::task::JoinHandle;
use futures::future;
use std::collections::HashMap;

#[derive(Debug)]
pub struct TargetRunStates<'a> {
    targets: &'a HashMap<TargetId, Target>,
    build_states: HashMap<TargetId, TargetRunState>,
}

impl<'a> TargetRunStates<'a> {
    pub fn new(targets: &'a HashMap<TargetId, Target>) -> Self {
        Self {
            targets,
            build_states: targets
                .keys()
                .cloned()
                .map(|target_id| (target_id, TargetRunState::new()))
                .collect(),
        }
    }

    pub fn set_invalidated(&mut self, target_id: &TargetId) {
        self.build_states
            .get_mut(target_id)
            .unwrap()
            .run_invalidated();
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

    pub fn set_run_started(&mut self, target_id: &TargetId) {
        self.build_states.get_mut(target_id).unwrap().run_started();
    }

    pub async fn set_finished(
        &mut self,
        target_id: &TargetId,
        result: &IncrementalRunResult<Result<()>>,
    ) {
        let target_build_state = self.build_states.get_mut(target_id).unwrap();
        if let IncrementalRunResult::Run(Err(_)) = result {
            target_build_state.run_failed().await;
        } else {
            target_build_state.run_succeeded().await;
        }
    }

    pub fn list_ready_to_run_targets(&self) -> Vec<TargetId> {
        self.build_states
            .iter()
            .filter(|(_target_id, build_state)| {
                build_state.to_run && build_state.build_handles.is_none()
            })
            .map(|(target_id, _build_state)| target_id)
            .filter(|&target_id| self.has_all_dependencies_built(target_id))
            .cloned()
            .collect()
    }

    fn has_all_dependencies_built(&self, target_id: &TargetId) -> bool {
        let target = &self.targets[target_id];

        target.dependencies().iter().all(|dependency_id| {
            self.build_states[dependency_id].done && self.has_all_dependencies_built(dependency_id)
        })
    }

    pub fn all_are_built(&self) -> bool {
        self.build_states
            .values()
            .all(|build_state| build_state.done)
    }

    pub async fn cancel_all_builds(&mut self) {
        let cancellation_futures = self
            .build_states
            .iter_mut()
            .filter(|(_target_id, build_state)| build_state.build_handles.is_some())
            .map(|(target_id, build_state)| async move {
                log::debug!("{} - Cancelling build", target_id);
                build_state.cancel_run().await;
            });

        future::join_all(cancellation_futures).await;
        log::debug!("All build processes successfully cancelled");
    }
}

#[derive(Debug)]
struct TargetRunState {
    to_run: bool,
    build_handles: Option<(JoinHandle<()>, Sender<BuildCancellationMessage>)>,
    done: bool,
}

impl TargetRunState {
    pub fn new() -> Self {
        Self {
            to_run: true,
            build_handles: None,
            done: false,
        }
    }

    pub fn run_invalidated(&mut self) {
        self.to_run = true;
        self.done = false;
    }

    pub fn build_started(
        &mut self,
        build_thread: JoinHandle<()>,
        build_cancellation_sender: Sender<BuildCancellationMessage>,
    ) {
        self.to_run = false;
        self.build_handles = Some((build_thread, build_cancellation_sender));
        self.done = false;
    }

    pub fn run_started(&mut self) {
        self.to_run = false;
        self.build_handles = None;
        self.done = false;
    }

    pub async fn run_succeeded(&mut self) {
        self.join_build_thread().await;
        self.done = !self.to_run;
    }

    pub async fn run_failed(&mut self) {
        self.join_build_thread().await;
        self.done = false;
    }

    async fn cancel_run(&mut self) {
        if self.build_handles.is_some() {
            let (build_thread, build_cancellation_sender) =
                std::mem::replace(&mut self.build_handles, None).unwrap();
            build_cancellation_sender
                .send(BuildCancellationMessage::CancelBuild)
                .await;
            build_thread.await;
        }
    }

    async fn join_build_thread(&mut self) {
        if self.build_handles.is_some() {
            let (build_thread, _build_cancellation_sender) =
                std::mem::replace(&mut self.build_handles, None).unwrap();
            build_thread.await;
        }
    }
}
