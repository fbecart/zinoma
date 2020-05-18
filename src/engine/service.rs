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

    pub fn restart_service(&mut self, target: &Target) -> Result<()> {
        if let Some(Some(service_process)) = self.service_processes.get_mut(target.id) {
            log::trace!("{} - Stopping service", target.name);
            service_process.kill().with_context(|| format!("Failed to kill service {}", target.name))?;
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
        for service_process in self.service_processes.iter_mut() {
            if let Some(service_process) = service_process {
                service_process.kill().unwrap_or_else(|e| {
                    println!("Failed to send Kill signal to running process: {}", e)
                });
            }
        }
    }
}
