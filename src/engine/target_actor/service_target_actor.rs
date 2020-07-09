use super::{TargetActorInputMessage, TargetActorOutputMessage, TargetInvalidatedMessage};
use crate::domain::{ServiceTarget, TargetId};
use crate::{run_script, TerminationMessage};
use anyhow::{Context, Result};
use async_std::prelude::*;
use async_std::sync::{Receiver, Sender};
use async_std::task;
use futures::FutureExt;
use std::collections::HashSet;
use std::iter::FromIterator;
use std::mem;
use std::process::{Child, Stdio};
pub struct ServiceTargetActor {
    target: ServiceTarget,
    termination_events: Receiver<TerminationMessage>,
    target_invalidated_events: Receiver<TargetInvalidatedMessage>,
    target_actor_input_receiver: Receiver<TargetActorInputMessage>,
    target_actor_output_sender: Sender<TargetActorOutputMessage>,
    to_execute: bool,
    executed: bool,
    dependencies: HashSet<TargetId>,
    unavailable_dependencies: HashSet<TargetId>,
    service_process: Option<Child>,
}

impl ServiceTargetActor {
    pub fn new(
        target: ServiceTarget,
        termination_events: Receiver<TerminationMessage>,
        target_invalidated_events: Receiver<TargetInvalidatedMessage>,
        target_actor_input_receiver: Receiver<TargetActorInputMessage>,
        target_actor_output_sender: Sender<TargetActorOutputMessage>,
    ) -> Self {
        let dependencies = HashSet::from_iter(target.metadata.dependencies.iter().cloned());
        let unavailable_dependencies = dependencies.clone();

        Self {
            target,
            termination_events,
            target_invalidated_events,
            target_actor_input_receiver,
            target_actor_output_sender,
            to_execute: true,
            executed: false,
            dependencies,
            unavailable_dependencies,
            service_process: None,
        }
    }

    pub async fn run(mut self) {
        loop {
            if self.to_execute && self.unavailable_dependencies.is_empty() {
                self.to_execute = false;
                self.executed = false;

                match self.restart_service().await {
                    Ok(()) => {
                        self.executed = !self.to_execute;

                        if self.executed {
                            let target_id = self.target.metadata.id.clone();
                            let msg = TargetActorOutputMessage::TargetAvailable(target_id);
                            self.target_actor_output_sender.send(msg).await;
                        }
                    }
                    Err(e) => {
                        self.executed = false;

                        let target_id = self.target.metadata.id.clone();
                        let msg = TargetActorOutputMessage::TargetExecutionError(target_id, e);
                        self.target_actor_output_sender.send(msg).await;
                    }
                }
            }

            futures::select! {
                _ = self.termination_events.next().fuse() => break,
                _ = self.target_invalidated_events.next().fuse() => self.invalidate().await,
                message = self.target_actor_input_receiver.next().fuse() => {
                    match message.unwrap() {
                        TargetActorInputMessage::TargetAvailable(target_id) => {
                            self.unavailable_dependencies.remove(&target_id);
                        },
                        TargetActorInputMessage::TargetInvalidated(target_id) => {
                            if self.dependencies.contains(&target_id) {
                                self.unavailable_dependencies.insert(target_id);
                                self.invalidate().await
                            }
                        }
                    }
                }
            }
        }

        self.stop_service().await;
    }

    async fn invalidate(&mut self) {
        if !self.to_execute {
            self.to_execute = true;
            self.executed = false;

            let target_id = self.target.metadata.id.clone();
            let msg = TargetActorOutputMessage::TargetInvalidated(target_id);
            self.target_actor_output_sender.send(msg).await;
        }
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
