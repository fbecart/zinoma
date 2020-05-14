use crate::domain::Target;
use anyhow::{Context, Result};
use crossbeam::channel::{unbounded, Receiver, Sender};
use crossbeam::thread::Scope;
use run_script::{IoOptions, ScriptOptions};

pub struct ServicesRunner {
    tx_channels: Vec<Option<Sender<RunSignal>>>,
}

impl ServicesRunner {
    pub fn new(targets: &[Target]) -> Self {
        Self {
            tx_channels: vec![None; targets.len()],
        }
    }

    pub fn restart_service<'a>(&mut self, scope: &Scope<'a>, target: &'a Target) -> Result<()> {
        if target.service.is_some() {
            // If already running, send a kill signal.
            if let Some(service_tx) = &self.tx_channels[target.id] {
                service_tx
                    .send(RunSignal::Kill)
                    .with_context(|| "Failed to send Kill signal to running process")?;
            }

            let (service_tx, service_rx) = unbounded();
            self.tx_channels[target.id] = Some(service_tx);

            scope.spawn(move |_| run_target_service(target, service_rx).unwrap());
        }

        Ok(())
    }
}

fn run_target_service(target: &Target, rx: Receiver<RunSignal>) -> Result<()> {
    if let Some(script) = &target.service {
        log::info!("{} - Starting service", target.name);

        let script = format!("cd {}\n{}", (&target.path).to_str().unwrap(), script);

        let mut options = ScriptOptions::new();
        options.exit_on_error = true;
        options.output_redirection = IoOptions::Inherit;

        let mut handle = run_script::spawn(&script, &vec![], &options)
            .with_context(|| format!("Failed to start service {}", target.name))?;

        match rx.recv().with_context(|| "Receiver error")? {
            RunSignal::Kill => {
                log::trace!("{} - Stopping service", target.name);
                handle
                    .kill()
                    .with_context(|| format!("Failed to kill service {}", target.name))?;
            }
        }
    }

    Ok(())
}

enum RunSignal {
    Kill,
}
