mod build_state;
mod builder;
pub mod incremental;
mod service;
mod watcher;

use crate::domain::{Target, TargetId};
use anyhow::{Context, Result};
use build_state::TargetBuildStates;
use builder::build_target;
use crossbeam::channel::{tick, unbounded, Receiver, Sender};
use crossbeam::thread::Scope;
use incremental::{IncrementalRunResult, IncrementalRunner};
use service::ServicesRunner;
use std::time::Duration;
use watcher::TargetsWatcher;

pub struct Engine {
    targets: Vec<Target>,
    incremental_runner: IncrementalRunner,
}

impl Engine {
    pub fn new(targets: Vec<Target>, incremental_runner: IncrementalRunner) -> Self {
        Self {
            targets,
            incremental_runner,
        }
    }

    pub fn watch(self, termination_events: Receiver<()>) -> Result<()> {
        let watcher =
            TargetsWatcher::new(&self.targets).with_context(|| "Failed to set up file watcher")?;

        let mut services_runner = ServicesRunner::new(&self.targets);
        let (build_report_sender, build_report_events) = unbounded();
        let mut target_build_states = TargetBuildStates::new(&self.targets, build_report_events);

        let ticks = tick(Duration::from_millis(10));

        crossbeam::scope(|scope| -> Result<()> {
            loop {
                crossbeam_channel::select! {
                  recv(ticks) -> _ => {
                    let invalidated_builds = watcher
                        .get_invalidated_targets()
                        .with_context(|| "File watch error")?;
                    target_build_states.set_builds_invalidated(&invalidated_builds);

                    self.build_ready_targets(scope, &mut target_build_states, &build_report_sender, &termination_events);

                    if let Some(build_report) = target_build_states.get_finished_build()? {
                        let target = &self.targets[build_report.target_id];
                        if let IncrementalRunResult::Run(Err(e)) = build_report.result {
                            log::warn!("{} - {}", target.name, e);
                        } else {
                            services_runner.restart_service(target)?;
                        }
                    }
                  },
                  recv(termination_events) -> _ => {
                      services_runner.terminate_all_services();
                      break Ok(());
                  }
                }
            }
        })
        .map_err(|_| anyhow::anyhow!("Unknown crossbeam parallelism failure (thread panicked)"))?
    }

    pub fn build(self, termination_events: Receiver<()>) -> Result<()> {
        let mut services_runner = ServicesRunner::new(&self.targets);
        let (build_report_sender, build_report_events) = unbounded();
        let mut target_build_states = TargetBuildStates::new(&self.targets, build_report_events);

        let ticks = tick(Duration::from_millis(10));

        crossbeam::scope(|scope| {
            while !target_build_states.all_are_built() {
                crossbeam_channel::select! {
                  recv(ticks) -> _ => {
                    self.build_ready_targets(scope, &mut target_build_states, &build_report_sender, &termination_events);

                    if let Some(build_report) = target_build_states.get_finished_build()? {
                        if let IncrementalRunResult::Run(Err(e)) = build_report.result {
                            return Err(e);
                        }

                        let target = &self.targets[build_report.target_id];
                        services_runner.start_service(target)?;
                    }
                  },
                  recv(termination_events) -> _ => {
                      services_runner.terminate_all_services();
                      break;
                  }
                }
            }

            if services_runner.has_running_services() {
                termination_events
                    .recv()
                    .with_context(|| "Failed to listen to termination event")?;
                services_runner.terminate_all_services();
            }

            Ok(())
        })
        .map_err(|_| anyhow::anyhow!("Unknown crossbeam parallelism failure (thread panicked)"))?
    }

    fn build_ready_targets<'a, 's>(
        &'a self,
        scope: &Scope<'s>,
        target_build_states: &mut TargetBuildStates,
        build_report_sender: &Sender<BuildReport>,
        termination_events: &'s Receiver<()>,
    ) where
        'a: 's,
    {
        for &target_id in &target_build_states.get_ready_to_build_targets() {
            target_build_states.set_build_started(target_id);

            let target = self.targets.get(target_id).unwrap();
            let build_report_sender = build_report_sender.clone();
            scope.spawn(move |_| {
                let result = self
                    .incremental_runner
                    .run(&target, || {
                        let termination_events = termination_events.clone();
                        build_target(&target, termination_events)
                    })
                    .with_context(|| format!("{} - Build failed", target.name))
                    .unwrap();

                if let IncrementalRunResult::Skipped = result {
                    log::info!("{} - Build skipped (Not Modified)", target.name);
                }

                build_report_sender
                    .send(BuildReport::new(target.id, result))
                    .with_context(|| "Sender error")
                    .unwrap();
            });
        }
    }
}

pub struct BuildReport {
    pub target_id: TargetId,
    pub result: IncrementalRunResult<Result<()>>,
}

impl BuildReport {
    pub fn new(target_id: TargetId, result: IncrementalRunResult<Result<()>>) -> Self {
        Self { target_id, result }
    }
}
