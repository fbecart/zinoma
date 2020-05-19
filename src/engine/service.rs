use super::process;
use crate::domain::Target;
use anyhow::{Context, Result};
use run_script::{IoOptions, ScriptOptions};
use std::process::Child;

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
            log::trace!("{} - Stopping service", target.name);
            process::kill_and_wait(service_process)
                .with_context(|| format!("Failed to kill service {}", target.name))?;
        }

        self.start_service(target)
    }

    pub fn start_service(&mut self, target: &Target) -> Result<()> {
        if let Some(script) = &target.service {
            log::info!("{} - Starting service", target.name);

            let mut options = ScriptOptions::new();
            options.exit_on_error = true;
            options.output_redirection = IoOptions::Inherit;
            options.working_directory = Some(target.path.to_path_buf());

            let service_process = run_script::spawn(&script, &vec![], &options)
                .with_context(|| format!("Failed to start service {}", target.name))?;

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
