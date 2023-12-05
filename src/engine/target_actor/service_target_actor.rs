use super::{ActorInputMessage, ExecutionKind, TargetActorHelper};
use crate::domain::ServiceTarget;
use crate::run_script;
use anyhow::{Context, Result};
use async_process::Child;
use async_std::prelude::*;
use futures::FutureExt;
use std::process::Stdio;

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
            if self.helper.should_execute(ExecutionKind::Service) {
                self.helper.set_execution_started();

                match self.restart_service().await {
                    Ok(()) => self.helper.notify_success(ExecutionKind::Service).await,
                    Err(e) => self.helper.notify_execution_failed(e).await,
                }
            }

            // TODO Catch service execution failures
            futures::select! {
                _ = self.helper.termination_events.next().fuse() => break,
                _ = self.helper.target_invalidated_events.next().fuse() => {
                    self.helper.notify_invalidated(ExecutionKind::Service).await
                }
                message = self.helper.target_actor_input_receiver.next().fuse() => {
                    match message.unwrap() {
                        ActorInputMessage::Ok { kind, target_id, .. } => {
                            self.helper.unavailable_dependencies.get_mut(&kind).unwrap().remove(&target_id);
                        },
                        ActorInputMessage::Invalidated { kind, target_id } => {
                            self.helper.unavailable_dependencies.get_mut(&kind).unwrap().insert(target_id);
                            self.helper.notify_invalidated(ExecutionKind::Service).await
                        }
                        ActorInputMessage::Requested { kind: ExecutionKind::Build, requester } => {
                            let msg = ActorInputMessage::Ok {
                                kind: ExecutionKind::Build,
                                target_id: self.helper.target_id.clone(),
                                actual: false,
                            };
                            self.helper.send_to_actor(requester, msg).await
                        }
                        ActorInputMessage::Requested { kind: ExecutionKind::Service, requester } => {
                            let inserted = self.helper.requesters.get_mut(&ExecutionKind::Service).unwrap().insert(requester);

                            if inserted && self.helper.requesters[&ExecutionKind::Service].len() == 1 {
                                self.helper.request_dependencies(ExecutionKind::Build).await;
                                self.helper.request_dependencies(ExecutionKind::Service).await;
                            }
                        }
                        ActorInputMessage::Unrequested { kind, requester } => {
                            let was_last_requester = self.helper.handle_unrequested(kind, requester);

                            if was_last_requester && kind == ExecutionKind::Service {
                                self.helper.unrequest_dependencies(ExecutionKind::Build).await;
                                self.helper.unrequest_dependencies(ExecutionKind::Service).await;

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
            let mut running_service = self.service_process.take().unwrap();
            log::trace!("{} - Stopping service", target_id);
            if let Err(e) = running_service.kill() {
                log::warn!("{} - Failed to kill service: {}", target_id, e);
            }
            if let Err(e) = running_service.status().await {
                log::warn!("{} - Failed to await killed service: {}", target_id, e);
            }
        }
    }

    async fn restart_service(&mut self) -> Result<()> {
        self.stop_service().await;

        log::info!("{} - Starting service", self.target.metadata.id);

        let mut command =
            run_script::build_command(&self.target.run_script, &self.target.metadata.project_dir);
        command.stdout(Stdio::inherit()).stderr(Stdio::inherit());

        let service_process = command
            .spawn()
            .with_context(|| "Failed to start service".to_string())?;

        self.service_process = Some(service_process);

        Ok(())
    }
}
