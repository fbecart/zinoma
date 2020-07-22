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
    WatchOption,
};

pub struct Engine {
    targets: HashMap<TargetId, Target>,
}

impl Engine {
    pub fn new(targets: HashMap<TargetId, Target>) -> Self {
        Self { targets }
    }

    pub async fn watch(
        self,
        root_target_ids: Vec<TargetId>,
        mut termination_events: Receiver<TerminationMessage>,
    ) -> Result<()> {
        let (target_actor_join_handles, target_actor_handles, mut target_actor_output_events) =
            Self::launch_target_actors(self.targets, WatchOption::Enabled)?;

        for target_id in &root_target_ids {
            Self::request_target(&target_actor_handles[target_id]).await
        }

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

        Self::send_termination_message(&target_actor_handles).await;
        future::join_all(target_actor_join_handles).await;

        Ok(())
    }

    pub async fn execute_once(
        self,
        root_target_ids: Vec<TargetId>,
        mut termination_events: Receiver<TerminationMessage>,
    ) -> Result<()> {
        let (target_actor_join_handles, target_actor_handles, mut target_actor_output_events) =
            Self::launch_target_actors(self.targets, WatchOption::Disabled)?;

        for target_id in &root_target_ids {
            Self::request_target(&target_actor_handles[target_id]).await
        }

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

        Self::send_termination_message(&target_actor_handles).await;
        future::join_all(target_actor_join_handles).await;

        Ok(())
    }

    fn launch_target_actors(
        targets: HashMap<TargetId, Target>,
        watch_option: WatchOption,
    ) -> Result<(
        Vec<JoinHandle<()>>,
        HashMap<TargetId, TargetActorHandleSet>,
        Receiver<TargetActorOutputMessage>,
    )> {
        let (target_actor_output_sender, target_actor_output_events) =
            sync::channel(crate::DEFAULT_CHANNEL_CAP);

        let mut target_actor_handles = HashMap::with_capacity(targets.len());
        let mut join_handles = Vec::with_capacity(targets.len());
        for (target_id, target) in targets.into_iter() {
            let (join_handle, handles) = target_actor::launch_target_actor(
                target,
                watch_option,
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
