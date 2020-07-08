use crate::domain::{Target, TargetId};
use crate::engine::{builder, incremental, BuildCancellationMessage};
use crate::{run_script, TerminationMessage};
use anyhow::{Context, Error, Result};
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
    target_invalidated_events: Receiver<TargetInvalidatedMessage>,
    receiver: Receiver<TargetActorInputMessage>,
    target_execution_report_sender: Sender<TargetExecutionReportMessage>,
    to_execute: bool,
    executed: bool,
    unavailable_dependencies: HashSet<TargetId>,
    service_process: Option<Child>,
}

impl TargetActor {
    pub fn new(
        target: Target,
        termination_events: Receiver<TerminationMessage>,
        target_invalidated_events: Receiver<TargetInvalidatedMessage>,
        receiver: Receiver<TargetActorInputMessage>,
        target_execution_report_sender: Sender<TargetExecutionReportMessage>,
    ) -> Self {
        let unavailable_dependencies = target
            .dependencies()
            .iter()
            .cloned()
            .collect::<HashSet<_>>();

        Self {
            target,
            termination_events,
            target_invalidated_events,
            receiver,
            target_execution_report_sender,
            to_execute: true,
            executed: false,
            unavailable_dependencies,
            service_process: None,
        }
    }

    pub async fn run(mut self) {
        loop {
            if let MaybeInterrupted::Interrupted = self.maybe_execute_target().await {
                break;
            }

            futures::select! {
                _ = self.termination_events.next().fuse() => break,
                _ = self.target_invalidated_events.next().fuse() => {
                    // TODO Should cascade to targets which depend on this one
                    self.to_execute = true;
                    self.executed = false;
                }
                message = self.receiver.next().fuse() => {
                    match message.unwrap() {
                        TargetActorInputMessage::TargetOutputAvailable(target_id) => {
                            self.unavailable_dependencies.remove(&target_id);
                        },
                    }
                }
            }
        }

        self.stop_service().await;
    }

    async fn stop_service(&mut self) {
        if self.service_process.is_some() {
            let target_id = self.target.id().clone();
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

    async fn maybe_execute_target(&mut self) -> MaybeInterrupted {
        if self.to_execute && self.unavailable_dependencies.is_empty() {
            self.to_execute = false;
            self.executed = false;

            let target_execution_result = self.execute_target().await;
            match target_execution_result {
                Ok(TargetExecutionResult::Success) => {
                    self.executed = !self.to_execute;
                    let msg = TargetExecutionReportMessage::TargetOutputAvailable(
                        self.target.id().clone(),
                    );
                    self.target_execution_report_sender.send(msg).await;
                    MaybeInterrupted::NotInterrupted
                }
                Err(e) => {
                    self.executed = false;
                    let msg = TargetExecutionReportMessage::TargetExecutionError(
                        self.target.id().clone(),
                        e,
                    );
                    self.target_execution_report_sender.send(msg).await;
                    MaybeInterrupted::NotInterrupted
                }
                Ok(TargetExecutionResult::InterruptedByTermination) => {
                    MaybeInterrupted::Interrupted
                }
            }
        } else {
            MaybeInterrupted::NotInterrupted
        }
    }

    async fn execute_target(&mut self) -> Result<TargetExecutionResult> {
        match &self.target.clone() {
            // TODO Remove clone
            Target::Build(build_target) => {
                let (build_cancellation_sender, build_cancellation_events) = sync::channel(1);
                let incremental_build = incremental::run(&self.target, || async {
                    builder::build_target(&build_target, build_cancellation_events.clone()).await
                });

                futures::select! {
                    _ = self.termination_events.next().fuse() => {
                        build_cancellation_sender.send(BuildCancellationMessage).await;
                        // TODO Uncomment
                        // incremental_build.await;
                        return Ok(TargetExecutionResult::InterruptedByTermination);
                    },
                    result = incremental_build.fuse() => {
                        // Why unwrap?
                        let result = result.with_context(|| format!("{} - Failed to evaluate target input/output", self.target)).unwrap();
                        match result {
                            IncrementalRunResult::Run(Err(e)) => return Err(e),
                            IncrementalRunResult::Skipped => {
                                log::info!("{} - Build skipped (Not Modified)", self.target);
                            },
                            IncrementalRunResult::Run(Ok(_)) => {
                                // TODO Why spreading logs between here and builder?
                            },
                        }
                        if let IncrementalRunResult::Run(Err(e)) = result {
                            return Err(e);
                        }
                    }
                }
            }
            Target::Service(service_target) => {
                self.stop_service().await;

                log::info!("{} - Starting service", service_target.metadata.id);

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

// TODO Rename
pub enum TargetActorInputMessage {
    TargetOutputAvailable(TargetId),
}

pub enum TargetExecutionReportMessage {
    TargetExecutionError(TargetId, Error),
    TargetOutputAvailable(TargetId),
}

enum TargetExecutionResult {
    InterruptedByTermination,
    Success,
}

enum MaybeInterrupted {
    Interrupted,
    NotInterrupted,
}

#[derive(Debug)]
pub struct TargetInvalidatedMessage;
