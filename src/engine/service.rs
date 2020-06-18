use crate::domain::{Target, TargetId, TargetType};
use crate::run_script;
use anyhow::{Context, Result};
use std::collections::HashSet;
use std::process::{Child, Stdio};

pub struct ServicesRunner {
    service_processes: Vec<Option<Child>>,
}

impl ServicesRunner {
    pub fn new(targets: &[Target]) -> Self {
        Self {
            service_processes: (0..targets.len()).map(|_| None).collect(),
        }
    }

    pub fn list_running_services(&self) -> Vec<TargetId> {
        self.service_processes
            .iter()
            .enumerate()
            .filter_map(|(target_id, process)| process.as_ref().map(|_| target_id))
            .collect::<Vec<_>>()
    }

    pub fn restart_service(&mut self, target: &Target) -> Result<()> {
        if let Some(Some(service_process)) = self.service_processes.get_mut(target.id) {
            log::trace!("{} - Stopping service", target);
            service_process
                .kill()
                .and_then(|_| service_process.wait())
                .with_context(|| format!("Failed to kill service {}", target))?;
        }

        self.start_service(target)
    }

    pub fn start_service(&mut self, target: &Target) -> Result<()> {
        if let TargetType::Service { run_script, .. } = &target.target_type {
            log::info!("{} - Starting service", target);

            let service_process = run_script::build_command(&run_script, &target.project_dir)
                .stdout(Stdio::inherit())
                .stderr(Stdio::inherit())
                .spawn()
                .with_context(|| format!("Failed to start service {}", target))?;

            self.service_processes[target.id] = Some(service_process);
        }

        Ok(())
    }

    pub fn terminate_all_services(&mut self) {
        self.terminate_services(&self.list_running_services());
    }

    pub fn terminate_services(&mut self, services: &[TargetId]) {
        for &target_id in services {
            if let Some(child_process) = self.service_processes.get_mut(target_id).unwrap() {
                if let Err(e) = child_process.kill() {
                    log::warn!("Failed to kill service: {}", e)
                }
            }
        }

        for &target_id in services {
            if let Some(child_process) = self.service_processes.get_mut(target_id).unwrap() {
                if let Err(e) = child_process.wait() {
                    log::warn!("Failed to wait for service termination: {}", e)
                }
            }

            self.service_processes.remove(target_id);
        }
    }
}

/// List targets of the service graph.
///
/// Returns the target IDs of the services, omitting those that are only required by build targets.
pub fn get_service_graph_targets(
    targets: &[Target],
    root_target_ids: &[TargetId],
) -> HashSet<TargetId> {
    root_target_ids
        .iter()
        .fold(HashSet::new(), |mut service_ids, &target_id| {
            let target = targets.get(target_id).unwrap();

            match target.target_type {
                TargetType::Service { .. } => {
                    service_ids.insert(target_id);
                    service_ids = service_ids
                        .union(&get_service_graph_targets(targets, &target.dependencies))
                        .cloned()
                        .collect();
                }
                TargetType::Aggregate { .. } => {
                    service_ids = service_ids
                        .union(&get_service_graph_targets(targets, &target.dependencies))
                        .cloned()
                        .collect();
                }
                TargetType::Build { .. } => {}
            }

            service_ids
        })
}
