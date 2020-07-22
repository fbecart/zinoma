pub mod builder;
pub mod incremental;
mod target_actor;
mod watcher;

use crate::domain::{Target, TargetId};
use crate::TerminationMessage;
use anyhow::{Context, Result};
use async_std::prelude::*;
use async_std::sync::{self, Receiver};
use async_std::task::JoinHandle;
use futures::{future, FutureExt};
use std::collections::{HashMap, HashSet};
use target_actor::{
    ActorId, ActorInputMessage, ExecutionKind, TargetActorHandleSet, TargetActorOutputMessage,
};

pub struct Engine {
    targets: HashMap<TargetId, Target>,
    watch_option: WatchOption,
}

impl Engine {
    pub fn new(targets: HashMap<TargetId, Target>, watch_option: WatchOption) -> Self {
        Self {
            targets,
            watch_option,
        }
    }

    pub async fn run(
        self,
        root_target_ids: Vec<TargetId>,
        termination_events: Receiver<TerminationMessage>,
    ) -> Result<()> {
        let watch_option = self.watch_option.clone();
        let (target_actor_join_handles, target_actor_handles, target_actor_output_events) =
            self.launch_target_actors()?;

        for target_id in &root_target_ids {
            Self::request_target(&target_actor_handles[target_id]).await
        }

        match watch_option {
            WatchOption::Enabled => {
                Self::watch(
                    termination_events,
                    target_actor_output_events,
                    &target_actor_handles,
                )
                .await
            }
            WatchOption::Disabled => {
                Self::execute_once(
                    &root_target_ids,
                    termination_events,
                    target_actor_output_events,
                    &target_actor_handles,
                )
                .await?
            }
        }

        Self::send_termination_message(&target_actor_handles).await;
        future::join_all(target_actor_join_handles).await;

        Ok(())
    }

    async fn watch(
        mut termination_events: Receiver<TerminationMessage>,
        mut target_actor_output_events: Receiver<TargetActorOutputMessage>,
        target_actor_handles: &HashMap<TargetId, TargetActorHandleSet>,
    ) {
        loop {
            futures::select! {
                _ = termination_events.next().fuse() => break,
                target_actor_output = target_actor_output_events.next().fuse() => {
                    match target_actor_output.unwrap() {
                        TargetActorOutputMessage::TargetExecutionError(target_id, e) => {
                            log::warn!("{} - {}", target_id, e);
                        },
                        TargetActorOutputMessage::MessageActor { dest, msg } => {
                            if let ActorId::Target(target_id) = dest {
                                target_actor_handles[&target_id].target_actor_input_sender.send(msg).await
                            }
                        }
                    }
                }
            }
        }
    }

    async fn execute_once(
        root_target_ids: &[TargetId],
        mut termination_events: Receiver<TerminationMessage>,
        mut target_actor_output_events: Receiver<TargetActorOutputMessage>,
        target_actor_handles: &HashMap<TargetId, TargetActorHandleSet>,
    ) -> Result<()> {
        let unavailable_root_targets = root_target_ids.iter().cloned().collect::<HashSet<_>>();
        let mut unavailable_root_builds = unavailable_root_targets.clone();
        let mut unavailable_root_services = unavailable_root_targets;
        let mut service_root_targets = HashSet::new();
        let mut terminating = false;

        while !(terminating
            || unavailable_root_services.is_empty() && unavailable_root_builds.is_empty())
        {
            futures::select! {
                _ = termination_events.next().fuse() => terminating = true,
                target_actor_output = target_actor_output_events.next().fuse() => {
                    match target_actor_output.unwrap() {
                        TargetActorOutputMessage::TargetExecutionError(target_id, e) => {
                            log::error!("{} - {}", target_id, e);
                            terminating = true
                        },
                        TargetActorOutputMessage::MessageActor { dest, msg } => match dest {
                            ActorId::Target(target_id) => {
                                target_actor_handles[&target_id].target_actor_input_sender.send(msg).await;
                            }
                            ActorId::Root => match msg {
                                ActorInputMessage::Ok { kind: ExecutionKind::Build, target_id, .. } => {
                                    unavailable_root_builds.remove(&target_id);
                                },
                                ActorInputMessage::Ok { kind: ExecutionKind::Service, target_id, actual } => {
                                    unavailable_root_services.remove(&target_id);

                                    if actual {
                                        service_root_targets.insert(target_id);
                                    }
                                },
                                _ => {},
                            }
                        }
                    }
                }
            }
        }

        if !terminating && !service_root_targets.is_empty() {
            // Wait for termination event
            termination_events
                .recv()
                .await
                .with_context(|| "Failed to listen to termination event".to_string())?;
        }

        Ok(())
    }

    fn launch_target_actors(
        self,
    ) -> Result<(
        Vec<JoinHandle<()>>,
        HashMap<TargetId, TargetActorHandleSet>,
        Receiver<TargetActorOutputMessage>,
    )> {
        let (target_actor_output_sender, target_actor_output_events) =
            sync::channel(crate::DEFAULT_CHANNEL_CAP);

        let mut target_actor_handles = HashMap::with_capacity(self.targets.len());
        let mut join_handles = Vec::with_capacity(self.targets.len());
        for (target_id, target) in self.targets.into_iter() {
            let (join_handle, handles) = target_actor::launch_target_actor(
                target,
                self.watch_option,
                target_actor_output_sender.clone(),
            )?;
            join_handles.push(join_handle);
            target_actor_handles.insert(target_id, handles);
        }

        Ok((
            join_handles,
            target_actor_handles,
            target_actor_output_events,
        ))
    }

    async fn send_termination_message(
        target_actor_handles: &HashMap<TargetId, TargetActorHandleSet>,
    ) {
        log::debug!("Terminating all targets");
        for handles in target_actor_handles.values() {
            handles.termination_sender.send(TerminationMessage).await;
        }
    }

    async fn request_target(handles: &TargetActorHandleSet) {
        for &kind in &[ExecutionKind::Build, ExecutionKind::Service] {
            let build_msg = ActorInputMessage::Requested {
                kind,
                requester: ActorId::Root,
            };
            handles.target_actor_input_sender.send(build_msg).await;
        }
    }
}

#[derive(Copy, Clone)]
pub enum WatchOption {
    Enabled,
    Disabled,
}
