use crate::domain::{ServiceTarget, Target, TargetId};
use crate::run_script;
use anyhow::{Context, Result};
use async_std::task;
use futures::future;
use std::collections::{HashMap, HashSet};
use std::process::{Child, Stdio};

pub struct ServicesRunner {
    service_processes: HashMap<TargetId, Child>,
}

impl ServicesRunner {
    pub fn new() -> Self {
        Self {
            service_processes: HashMap::new(),
        }
    }

    pub fn list_running_services(&self) -> Vec<TargetId> {
        self.service_processes.keys().cloned().collect::<Vec<_>>()
    }

    pub async fn restart_service(&mut self, target: &ServiceTarget) -> Result<()> {
        if let Some((target_id, mut service_process)) =
            self.service_processes.remove_entry(&target.metadata.id)
        {
            log::trace!("{} - Stopping service", target_id);
            task::spawn_blocking(move || {
                service_process.kill().and_then(|_| service_process.wait())
            })
            .await
            .with_context(|| format!("{} - Failed to stop service", target_id))?;
        }

        self.start_service(target).await
    }

    pub async fn start_service(&mut self, target: &ServiceTarget) -> Result<()> {
        log::info!("{} - Starting service", target);

        let mut command =
            run_script::build_command(&target.run_script, &target.metadata.project_dir);
        command.stdout(Stdio::inherit()).stderr(Stdio::inherit());

        let service_process = task::spawn_blocking(move || command.spawn())
            .await
            .with_context(|| format!("Failed to start service {}", target))?;

        self.service_processes
            .insert(target.metadata.id.clone(), service_process);

        Ok(())
    }

    pub async fn terminate_all_services(&mut self) {
        self.terminate_services(&self.list_running_services()).await;
    }

    pub async fn terminate_services(&mut self, services: &[TargetId]) {
        let processes = services
            .iter()
            .flat_map(|target_id| self.service_processes.remove_entry(target_id));

        future::join_all(
            processes.map(|(target_id, mut service_process)| async move {
                task::spawn_blocking(move || {
                    log::trace!("{} - Stopping service", target_id);
                    if let Err(e) = service_process.kill().and_then(|_| service_process.wait()) {
                        log::warn!("{} - Failed to stop service: {}", target_id, e);
                    }
                })
                .await
            }),
        )
        .await;
    }
}

/// List targets of the service graph.
///
/// Returns the target IDs of the services, omitting those that are only required by build targets.
pub fn get_service_graph_targets(
    targets: &HashMap<TargetId, Target>,
    root_target_ids: &[TargetId],
) -> HashSet<TargetId> {
    root_target_ids
        .iter()
        .fold(HashSet::new(), |mut service_ids, target_id| {
            let target = targets.get(&target_id).unwrap();

            match target {
                Target::Service(_) => {
                    service_ids.insert(target_id.clone());
                    service_ids = service_ids
                        .union(&get_service_graph_targets(targets, target.dependencies()))
                        .cloned()
                        .collect();
                }
                Target::Aggregate(_) => {
                    service_ids = service_ids
                        .union(&get_service_graph_targets(targets, target.dependencies()))
                        .cloned()
                        .collect();
                }
                Target::Build(_) => {}
            }

            service_ids
        })
}
