use super::{TargetActorHelper, TargetActorInputMessage};
use crate::domain::AggregateTarget;
use async_std::prelude::*;
use futures::FutureExt;
pub struct AggregateTargetActor {
    _target: AggregateTarget,
    helper: TargetActorHelper,
}

impl AggregateTargetActor {
    pub fn new(target: AggregateTarget, helper: TargetActorHelper) -> Self {
        Self {
            _target: target,
            helper,
        }
    }

    pub async fn run(mut self) {
        loop {
            if self.helper.is_ready_to_execute() {
                self.helper.set_execution_started();
                self.helper.notify_execution_succeeded().await;
            }

            futures::select! {
                _ = self.helper.termination_events.next().fuse() => break,
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
    }
}
