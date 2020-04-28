use crate::target::Target;
use crossbeam::channel::{unbounded, Receiver, Sender};
use crossbeam::thread::Scope;
use duct::cmd;

pub struct ServicesRunner {
    tx_channels: Vec<Option<Sender<RunSignal>>>,
}

impl ServicesRunner {
    pub fn new(targets: &[Target]) -> Self {
        Self {
            tx_channels: vec![None; targets.len()],
        }
    }

    pub fn restart_service<'a>(
        &mut self,
        scope: &Scope<'a>,
        target: &'a Target,
    ) -> Result<(), String> {
        if target.service.is_some() {
            // If already running, send a kill signal.
            if let Some(service_tx) = &self.tx_channels[target.id] {
                service_tx
                    .send(RunSignal::Kill)
                    .map_err(|e| format!("Failed to send Kill signal to running process: {}", e))?;
            }

            let (service_tx, service_rx) = unbounded();
            self.tx_channels[target.id] = Some(service_tx);

            scope.spawn(move |_| run_target_service(target, service_rx).unwrap());
        }

        Ok(())
    }
}

fn run_target_service(target: &Target, rx: Receiver<RunSignal>) -> Result<(), String> {
    if let Some(command) = &target.service {
        log::info!("{} - Command: \"{}\" - Run", target.name, command);
        let handle = cmd!("/bin/sh", "-c", command)
            .dir(&target.path)
            .stderr_to_stdout()
            .start()
            .map_err(|e| format!("Failed to run command {}: {}", command, e))?;

        match rx.recv() {
            Ok(RunSignal::Kill) => {
                log::trace!("{} - Killing process", target.name);
                handle
                    .kill()
                    .map_err(|e| format!("Failed to kill process {}: {}", command, e))
            }
            Err(e) => Err(format!("Receiver error: {}", e)),
        }?
    }

    Ok(())
}

enum RunSignal {
    Kill,
}
