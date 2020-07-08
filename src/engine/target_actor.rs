use crate::domain::{Target, TargetId};
use crate::engine::{builder, incremental, BuildCancellationMessage};
use crate::{run_script, TerminationMessage};
use anyhow::{Context, Result};
use async_std::prelude::*;
use async_std::sync::{self, Receiver, Sender};
use async_std::task;
use futures::FutureExt;
use incremental::IncrementalRunResult;
use std::collections::HashSet;
use std::mem;
use std::process::{Child, Stdio};

pub struct TargetActor {
    target: Target,
    termination_events: Receiver<TerminationMessage>,
    receiver: Receiver<TargetActorInputMessage>,
    target_execution_status_sender: Sender<TargetExecutionStatusMessage>,
    to_execute: bool,
    executed: bool,
    unavailable_dependencies: HashSet<TargetId>,
    service_process: Option<Child>,
}

impl TargetActor {
    pub fn new(
        target: Target,
        termination_events: Receiver<TerminationMessage>,
        receiver: Receiver<TargetActorInputMessage>,
        target_execution_status_sender: Sender<TargetExecutionStatusMessage>,
    ) -> Self {
        let unavailable_dependencies = target
            .dependencies()
            .iter()
            .cloned()
            .collect::<HashSet<_>>();

        Self {
            target,
            termination_events,
            receiver,
            target_execution_status_sender: target_execution_status_sender,
            to_execute: true,
            executed: false,
            unavailable_dependencies,
            service_process: None,
        }
    }

    pub async fn run(mut self) {
        loop {
            if let Some(Ok(TargetExecutionResult::InterruptedByTermination)) = self.maybe_execute_target().await {
                break;
            }

            futures::select! {
                _ = self.termination_events.next().fuse() => {
                    if self.service_process.is_some() {
                        let mut running_service = mem::replace(&mut self.service_process, None).unwrap();
                        log::trace!("{} - Stopping service", self.target);
                        task::spawn_blocking(move || {
                            if let Err(e) = running_service.kill().and_then(|_| running_service.wait()) {
                                log::warn!("{} - Failed to stop service: {}", self.target, e);
                            }
                        })
                        .await;
                    }

                    break;
                },
                message = self.receiver.next().fuse() => {
                    match message.unwrap() {
                        TargetActorInputMessage::TargetOutputAvailable(target_id) => {
                            self.unavailable_dependencies.remove(&target_id);
                        },
                        TargetActorInputMessage::TargetInvalidated => {
                            // TODO Should cascade to targets which depend on this one
                            self.to_execute = true;
                            self.executed = false;
                        },
                    }
                }
            }
        }
    }

    async fn maybe_execute_target(&mut self) -> Option<Result<TargetExecutionResult>> {
        if self.to_execute && self.unavailable_dependencies.is_empty() {
            self.to_execute = false;
            self.executed = false;

            let target_execution_result = self.execute_target().await;
            match &target_execution_result {
                Ok(TargetExecutionResult::Success) => {
                    self.executed = !self.to_execute;
                    let msg = TargetExecutionStatusMessage::TargetOutputAvailable(
                        self.target.id().clone(),
                    );
                    self.target_execution_status_sender.send(msg).await;
                }
                Err(e) => {
                    self.executed = false;
                    let msg = TargetExecutionStatusMessage::TargetExecutionError(
                        self.target.id().clone(),
                    );
                    self.target_execution_status_sender.send(msg).await;
                }
                Ok(TargetExecutionResult::InterruptedByTermination) => {}
            };

            return Some(target_execution_result);
        }

        return None;
    }

    async fn execute_target(&mut self) -> Result<TargetExecutionResult> {
        match &self.target {
            Target::Build(build_target) => {
                let (build_cancellation_sender, build_cancellation_events) = sync::channel(1);
                let incremental_build = incremental::run(&self.target, || async {
                    builder::build_target(&build_target, build_cancellation_events.clone()).await
                });

                futures::select! {
                    _ = self.termination_events.next().fuse() => {
                        build_cancellation_sender.send(BuildCancellationMessage::CancelBuild).await;
                        // TODO Uncomment
                        // incremental_build.await;
                        return Ok(TargetExecutionResult::InterruptedByTermination);
                    },
                    result = incremental_build.fuse() => {
                        // Why unwrap?
                        let result = result.with_context(|| format!("{} - Failed to evaluate target input/output", self.target)).unwrap();
                        if let IncrementalRunResult::Run(Err(e)) = result {
                            return Err(e);
                        }
                    }
                }
            }
            Target::Service(service_target) => {
                if self.service_process.is_some() {
                    let mut running_service =
                        mem::replace(&mut self.service_process, None).unwrap();
                    log::trace!("{} - Stopping service", service_target.metadata.id);
                    task::spawn_blocking(move || {
                        running_service.kill().and_then(|_| running_service.wait())
                    })
                    .await
                    .with_context(|| {
                        format!("{} - Failed to stop service", &service_target.metadata.id)
                    })?;
                }

                let mut command = run_script::build_command(
                    &service_target.run_script,
                    &service_target.metadata.project_dir,
                );
                command.stdout(Stdio::inherit()).stderr(Stdio::inherit());

                let service_process = task::spawn_blocking(move || command.spawn())
                    .await
                    .with_context(|| {
                        format!("{} - Failed to start service", &service_target.metadata.id)
                    })?;

                self.service_process = Some(service_process);
            }
            Target::Aggregate(_) => {}
        }

        Ok(TargetExecutionResult::Success)
    }
}

pub enum TargetActorInputMessage {
    TargetInvalidated,
    TargetOutputAvailable(TargetId),
}

pub enum TargetExecutionStatusMessage {
    TargetExecutionError(TargetId),
    TargetOutputAvailable(TargetId),
}

enum TargetExecutionResult {
    InterruptedByTermination,
    Success,
}
