use super::{ActorId, ActorInputMessage, TargetActorHelper};
use crate::domain::ServiceTarget;
use crate::run_script;
use anyhow::{Context, Result};
use async_std::prelude::*;
use async_std::task;
use futures::FutureExt;
use std::mem;
use std::process::{Child, Stdio};
pub struct ServiceTargetActor {
    target: ServiceTarget,
    helper: TargetActorHelper,
    service_process: Option<Child>,
}

impl ServiceTargetActor {
    pub fn new(target: ServiceTarget, helper: TargetActorHelper) -> Self {
        Self {
            target,
            helper,
            service_process: None,
        }
    }

    pub async fn run(mut self) {
        loop {
            if self.helper.to_execute
                && !self.helper.service_requesters.is_empty()
                && self.helper.unavailable_dependency_builds.is_empty()
                && self.helper.unavailable_dependency_services.is_empty()
            {
                self.helper.to_execute = false;
                self.helper.executed = false;

                match self.restart_service().await {
                    Ok(()) => {
                        self.helper.executed = !self.helper.to_execute;

                        if self.helper.executed {
                            let msg = ActorInputMessage::ServiceOk {
                                target_id: self.helper.target_id.clone(),
                                has_service: true,
                            };
                            self.helper.send_to_service_requesters(msg).await;
                        }
                    }
                    Err(e) => {
                        self.helper.executed = false;
                        self.helper.send_target_execution_error(e).await;
                    }
                }
            }

            futures::select! {
                _ = self.helper.termination_events.next().fuse() => break,
                _ = self.helper.target_invalidated_events.next().fuse() => {
                    self.helper.notify_service_invalidated().await
                }
                message = self.helper.target_actor_input_receiver.next().fuse() => {
                    match message.unwrap() {
                        ActorInputMessage::BuildOk { target_id } => {
                            self.helper.unavailable_dependency_builds.remove(&target_id);
                        },
                        ActorInputMessage::ServiceOk { target_id, .. } => {
                            self.helper.unavailable_dependency_services.remove(&target_id);
                        },
                        ActorInputMessage::BuildInvalidated { target_id } => {
                            self.helper.unavailable_dependency_builds.insert(target_id);
                            self.helper.notify_service_invalidated().await
                        }
                        ActorInputMessage::ServiceInvalidated { target_id } => {
                            self.helper.unavailable_dependency_services.insert(target_id);
                            self.helper.notify_service_invalidated().await
                        }
                        ActorInputMessage::BuildRequested { requester } => {
                            let msg = ActorInputMessage::BuildOk { target_id: self.helper.target_id.clone() };
                            self.helper.send_to_actor(requester, msg).await
                        }
                        ActorInputMessage::ServiceRequested { requester } => {
                            let inserted = self.helper.service_requesters.insert(requester);

                            if inserted && self.helper.service_requesters.len() == 1 {
                                self.helper.send_to_dependencies(ActorInputMessage::BuildRequested {
                                    requester: ActorId::Target(self.helper.target_id.clone()),
                                }).await;
                                self.helper.send_to_dependencies(ActorInputMessage::ServiceRequested {
                                    requester: ActorId::Target(self.helper.target_id.clone()),
                                }).await;
                            }
                        }
                        ActorInputMessage::BuildUnrequested { requester } => {}
                        ActorInputMessage::ServiceUnrequested { requester } => {
                            let removed = self.helper.service_requesters.remove(&requester);

                            if removed && self.helper.service_requesters.is_empty() {
                                self.helper.send_to_dependencies(ActorInputMessage::BuildUnrequested {
                                    requester: ActorId::Target(self.helper.target_id.clone()),
                                }).await;
                                self.helper.send_to_dependencies(ActorInputMessage::ServiceUnrequested {
                                    requester: ActorId::Target(self.helper.target_id.clone()),
                                }).await;

                                self.stop_service().await;
                            }
                        }
                    }
                }
            }
        }

        self.stop_service().await;
    }

    async fn stop_service(&mut self) {
        if self.service_process.is_some() {
            let target_id = self.target.metadata.id.clone();
            let mut running_service = mem::replace(&mut self.service_process, None).unwrap();
            log::trace!("{} - Stopping service", target_id);
            task::spawn_blocking(move || {
                if let Err(e) = running_service.kill().and_then(|_| running_service.wait()) {
                    log::warn!("{} - Failed to stop service: {}", target_id, e);
                }
            })
            .await;
        }
    }

    async fn restart_service(&mut self) -> Result<()> {
        self.stop_service().await;

        log::info!("{} - Starting service", self.target.metadata.id);

        let mut command =
            run_script::build_command(&self.target.run_script, &self.target.metadata.project_dir);
        command.stdout(Stdio::inherit()).stderr(Stdio::inherit());

        let service_process = task::spawn_blocking(move || command.spawn())
            .await
            .with_context(|| format!("{} - Failed to start service", &self.target.metadata.id))?;

        self.service_process = Some(service_process);

        Ok(())
    }
}
