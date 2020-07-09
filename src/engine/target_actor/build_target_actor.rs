use super::{TargetActorInputMessage, TargetActorOutputMessage, TargetInvalidatedMessage};
use crate::domain::{BuildTarget, Target, TargetId};
use crate::engine::{builder, incremental};
use crate::TerminationMessage;
use anyhow::{Context, Result};
use async_std::prelude::*;
use async_std::sync::{Receiver, Sender};
use builder::BuildCompletionReport;
use futures::FutureExt;
use incremental::IncrementalRunResult;
use std::collections::HashSet;
use std::iter::FromIterator;
pub struct BuildTargetActor {
    target: BuildTarget,
    termination_events: Receiver<TerminationMessage>,
    target_invalidated_events: Receiver<TargetInvalidatedMessage>,
    target_actor_input_receiver: Receiver<TargetActorInputMessage>,
    target_actor_output_sender: Sender<TargetActorOutputMessage>,
    to_execute: bool,
    executed: bool,
    dependencies: HashSet<TargetId>,
    unavailable_dependencies: HashSet<TargetId>,
}

impl BuildTargetActor {
    pub fn new(
        target: BuildTarget,
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
        }
    }

    pub async fn run(mut self) {
        loop {
            if self.to_execute && self.unavailable_dependencies.is_empty() {
                self.to_execute = false;
                self.executed = false;

                match self.build_target().await {
                    Ok(TargetExecutionResult::InterruptedByTermination) => break,
                    Ok(TargetExecutionResult::Success) => {
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

    async fn build_target(&mut self) -> Result<TargetExecutionResult> {
        // TODO Remove clone
        let target = Target::Build(self.target.clone());
        let result = incremental::run(&target, || async {
            builder::build_target(&self.target, self.termination_events.clone()).await
        })
        .await;

        // TODO Why unwrap?
        let result = result
            .with_context(|| format!("{} - Failed to evaluate target input/output", self.target))
            .unwrap();
        match result {
            IncrementalRunResult::Run(Err(e)) => return Err(e),
            IncrementalRunResult::Skipped => {
                log::info!("{} - Build skipped (Not Modified)", self.target);
            }
            IncrementalRunResult::Run(Ok(BuildCompletionReport::Completed)) => {
                // TODO Why spreading logs between here and builder?
            }
            IncrementalRunResult::Run(Ok(BuildCompletionReport::Aborted)) => {
                return Ok(TargetExecutionResult::InterruptedByTermination);
            }
        }

        Ok(TargetExecutionResult::Success)
    }
}

enum TargetExecutionResult {
    InterruptedByTermination,
    Success,
}
