use super::{TargetActorHelper, TargetActorInputMessage};
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
            if self.helper.is_ready_to_execute() {
                self.helper.set_execution_started();

                match self.restart_service().await {
                    Ok(()) => self.helper.notify_execution_succeeded().await,
                    Err(e) => self.helper.notify_execution_failed(e).await,
                }
            }

            futures::select! {
                _ = self.helper.termination_events.next().fuse() => break,
                _ = self.helper.target_invalidated_events.next().fuse() => {
                    self.helper.notify_target_invalidated().await
                }
                message = self.helper.target_actor_input_receiver.next().fuse() => {
                    match message.unwrap() {
                        TargetActorInputMessage::TargetAvailable(target_id) => {
                            self.helper.unavailable_dependencies.remove(&target_id);
                        },
                        TargetActorInputMessage::TargetInvalidated(target_id) => {
                            if self.helper.dependencies.contains(&target_id) {
                                self.helper.unavailable_dependencies.insert(target_id);
                                self.helper.notify_target_invalidated().await
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
