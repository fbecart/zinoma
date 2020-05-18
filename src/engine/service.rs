use crate::domain::Target;
use anyhow::{Context, Result};
use crossbeam::channel::{bounded, Receiver, Sender};
use crossbeam::thread::Scope;
use run_script::{IoOptions, ScriptOptions};

pub struct ServicesRunner {
    terminate_service_senders: Vec<Option<Sender<()>>>,
}

impl ServicesRunner {
    pub fn new(targets: &[Target]) -> Self {
        Self {
            terminate_service_senders: vec![None; targets.len()],
        }
    }

    pub fn restart_service(&mut self, scope: &Scope, target: Target) -> Result<()> {
        // If already running, send a kill signal.
        if let Some(terminate_service_sender) = &self.terminate_service_senders[target.id] {
            terminate_service_sender
                .send(())
                .with_context(|| "Failed to send Kill signal to running service")?;
        }

        self.start_service(scope, target)
    }

    pub fn start_service(&mut self, scope: &Scope, target: Target) -> Result<()> {
        if target.service.is_some() {
            let (terminate_service_sender, terminate_service_events) = bounded(0);
            self.terminate_service_senders[target.id] = Some(terminate_service_sender);

            scope.spawn(move |_| run_target_service(target, terminate_service_events).unwrap());
        }

        Ok(())
    }

    pub fn terminate_all_services(&mut self) {
        for terminate_service_sender in &self.terminate_service_senders {
            if let Some(terminate_service_sender) = terminate_service_sender {
                terminate_service_sender.send(()).unwrap_or_else(|e| {
                    println!("Failed to send Kill signal to running process: {}", e)
                });
            }
        }
    }
}

fn run_target_service(target: Target, terminate_service_events: Receiver<()>) -> Result<()> {
    if let Some(script) = &target.service {
        log::info!("{} - Starting service", target.name);

        let mut options = ScriptOptions::new();
        options.exit_on_error = true;
        options.output_redirection = IoOptions::Inherit;
        options.working_directory = Some(target.path.to_path_buf());

        let mut handle = run_script::spawn(&script, &vec![], &options)
            .with_context(|| format!("Failed to start service {}", target.name))?;

        // Wait for termination event
        terminate_service_events
            .recv()
            .with_context(|| "Receiver error")?;
        log::trace!("{} - Stopping service", target.name);
        handle
            .kill()
            .with_context(|| format!("Failed to kill service {}", target.name))?;
    }

    Ok(())
}
