use super::process;
use crate::domain::Target;
use crate::run_script;
use anyhow::{Context, Result};
use std::process::Child;
use std::process::Stdio;

pub struct ServicesRunner {
    service_processes: Vec<Option<Child>>,
}

impl ServicesRunner {
    pub fn new(targets: &[Target]) -> Self {
        Self {
            service_processes: (0..targets.len()).map(|_| None).collect(),
        }
    }

    pub fn has_running_services(&self) -> bool {
        self.service_processes.iter().any(Option::is_some)
    }

    pub fn restart_service(&mut self, target: &Target) -> Result<()> {
        if let Some(Some(service_process)) = self.service_processes.get_mut(target.id) {
            log::trace!("{} - Stopping service", target);
            process::kill_and_wait(service_process)
                .with_context(|| format!("Failed to kill service {}", target))?;
        }

        self.start_service(target)
    }

    pub fn start_service(&mut self, target: &Target) -> Result<()> {
        if let Some(script) = &target.service {
            log::info!("{} - Starting service", target);

            let service_process = run_script::build_command(&script, &target.project_dir)
                .stdout(Stdio::inherit())
                .stderr(Stdio::inherit())
                .spawn()
                .with_context(|| format!("Failed to stargt service {}", target))?;

            self.service_processes[target.id] = Some(service_process);
        }

        Ok(())
    }

    pub fn terminate_all_services(&mut self) {
        for service_process in self.service_processes.iter_mut().flatten() {
            service_process
                .kill()
                .unwrap_or_else(|e| println!("Failed to kill service: {}", e));
        }
        for service_process in self.service_processes.iter_mut().flatten() {
            service_process
                .wait()
                .map(|_exit_status| ())
                .unwrap_or_else(|e| println!("Failed to wait for service termination: {}", e));
        }
    }
}
