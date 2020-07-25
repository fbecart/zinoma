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
use sync::Sender;
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
        let (target_actor_output_sender, target_actor_output_events) =
            sync::channel(crate::DEFAULT_CHANNEL_CAP);

        let target_actors =
            TargetActors::launch(self.targets, target_actor_output_sender, self.watch_option)?;

        for target_id in &root_target_ids {
            target_actors.request_target(target_id).await;
        }

        let result = match self.watch_option {
            WatchOption::Enabled => {
                Self::watch(
                    termination_events,
                    target_actor_output_events,
                    &target_actors,
                )
                .await;
                Ok(())
            }
            WatchOption::Disabled => {
                Self::execute_once(
                    &root_target_ids,
                    termination_events,
                    target_actor_output_events,
                    &target_actors,
                )
                .await
            }
        };

        target_actors.terminate().await;

        result
    }

    async fn watch(
        mut termination_events: Receiver<TerminationMessage>,
        mut target_actor_output_events: Receiver<TargetActorOutputMessage>,
        target_actors: &TargetActors,
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
                                target_actors.send(&target_id, msg).await;
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
        target_actors: &TargetActors,
    ) -> Result<()> {
        let unavailable_root_targets = root_target_ids.iter().cloned().collect::<HashSet<_>>();
        let mut unavailable_root_builds = unavailable_root_targets.clone();
        let mut unavailable_root_services = unavailable_root_targets;
        let mut service_root_targets = HashSet::new();
        let mut termination_event_received = false;

        while !(termination_event_received
            || unavailable_root_services.is_empty() && unavailable_root_builds.is_empty())
        {
            futures::select! {
                _ = termination_events.next().fuse() => termination_event_received = true,
                target_actor_output = target_actor_output_events.next().fuse() => {
                    match target_actor_output.unwrap() {
                        TargetActorOutputMessage::TargetExecutionError(target_id, e) => {
                            return Err(e.context(format!("An issue occurred with target {}", target_id)));
                        },
                        TargetActorOutputMessage::MessageActor { dest, msg } => match dest {
                            ActorId::Target(target_id) => {
                                target_actors.send(&target_id, msg).await;
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

        if !termination_event_received && !service_root_targets.is_empty() {
            // Wait for termination event
            termination_events
                .recv()
                .await
                .with_context(|| "Failed to listen to termination event".to_string())?;
        }

        Ok(())
    }
}

struct TargetActors {
    target_actor_handles: HashMap<TargetId, TargetActorHandleSet>,
    target_actor_join_handles: Vec<JoinHandle<()>>,
}

impl TargetActors {
    fn launch(
        targets: HashMap<TargetId, Target>,
        target_actor_output_sender: Sender<TargetActorOutputMessage>,
        watch_option: WatchOption,
    ) -> Result<Self> {
        let launch_target_actor = |(target_id, target)| {
            target_actor::launch_target_actor(
                target,
                watch_option,
                target_actor_output_sender.clone(),
            )
            .map(|(join_handle, handles)| ((target_id, handles), join_handle))
        };

        let (target_actor_handles, target_actor_join_handles): (HashMap<_, _>, Vec<_>) = targets
            .into_iter()
            .map(launch_target_actor)
            .collect::<Result<Vec<_>>>()?
            .into_iter()
            .unzip();

        Ok(Self {
            target_actor_handles,
            target_actor_join_handles,
        })
    }

    async fn send(&self, target_id: &TargetId, msg: ActorInputMessage) {
        self.target_actor_handles[target_id]
            .target_actor_input_sender
            .send(msg)
            .await
    }

    async fn request_target(&self, target_id: &TargetId) {
        let handles = &self.target_actor_handles[target_id];
        for &kind in &[ExecutionKind::Build, ExecutionKind::Service] {
            let build_msg = ActorInputMessage::Requested {
                kind,
                requester: ActorId::Root,
            };
            handles.target_actor_input_sender.send(build_msg).await;
        }
    }

    async fn terminate(self) {
        Self::send_termination_message(&self.target_actor_handles).await;
        future::join_all(self.target_actor_join_handles).await;
    }

    async fn send_termination_message(
        target_actor_handles: &HashMap<TargetId, TargetActorHandleSet>,
    ) {
        log::debug!("Terminating all targets");
        for handles in target_actor_handles.values() {
            handles.termination_sender.send(TerminationMessage).await;
        }
    }
}

#[derive(Copy, Clone)]
pub enum WatchOption {
    Enabled,
    Disabled,
}

impl From<bool> for WatchOption {
    fn from(value: bool) -> Self {
        if value {
            WatchOption::Enabled
        } else {
            WatchOption::Disabled
        }
    }
}
