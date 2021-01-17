use super::target_actor::{
    self, ActorId, ActorInputMessage, ExecutionKind, TargetActorHandleSet, TargetActorOutputMessage,
};
use super::WatchOption;
use crate::domain::{Target, TargetId};
use crate::TerminationMessage;
use anyhow::Result;
use async_std::channel::Sender;
use async_std::task::JoinHandle;
use futures::future;
use std::collections::HashMap;

pub struct TargetActors {
    targets: HashMap<TargetId, Target>,
    target_actor_output_sender: Sender<TargetActorOutputMessage>,
    watch_option: WatchOption,
    target_actor_handles: HashMap<TargetId, TargetActorHandleSet>,
    target_actor_join_handles: Vec<JoinHandle<()>>,
}

impl TargetActors {
    pub fn new(
        targets: HashMap<TargetId, Target>,
        target_actor_output_sender: Sender<TargetActorOutputMessage>,
        watch_option: WatchOption,
    ) -> Self {
        Self {
            targets,
            target_actor_output_sender,
            watch_option,
            target_actor_handles: HashMap::new(),
            target_actor_join_handles: Vec::new(),
        }
    }

    fn get_target_actor_handles<'a>(
        &'a mut self,
        target_id: &TargetId,
    ) -> Result<&'a TargetActorHandleSet> {
        if !&self.target_actor_handles.contains_key(target_id) {
            let (owned_target_id, target) = self.targets.remove_entry(target_id).unwrap();
            let (join_handle, handles) = target_actor::launch_target_actor(
                target,
                self.watch_option,
                self.target_actor_output_sender.clone(),
            )?;
            self.target_actor_handles.insert(owned_target_id, handles);
            self.target_actor_join_handles.push(join_handle);
        }

        Ok(&self.target_actor_handles[target_id])
    }

    pub async fn send(&mut self, target_id: &TargetId, msg: ActorInputMessage) -> Result<()> {
        let _ = self
            .get_target_actor_handles(target_id)?
            .target_actor_input_sender
            .send(msg)
            .await;

        Ok(())
    }

    pub async fn request_target(&mut self, target_id: &TargetId) -> Result<()> {
        let handles = self.get_target_actor_handles(target_id)?;
        for &kind in &[ExecutionKind::Build, ExecutionKind::Service] {
            let build_msg = ActorInputMessage::Requested {
                kind,
                requester: ActorId::Root,
            };
            let _ = handles.target_actor_input_sender.send(build_msg).await;
        }

        Ok(())
    }

    pub async fn terminate(self) {
        Self::send_termination_message(&self.target_actor_handles).await;
        future::join_all(self.target_actor_join_handles).await;
    }

    async fn send_termination_message(
        target_actor_handles: &HashMap<TargetId, TargetActorHandleSet>,
    ) {
        log::debug!("Terminating all targets");
        for handles in target_actor_handles.values() {
            let _ = handles.termination_sender.send(TerminationMessage).await;
        }
    }
}
