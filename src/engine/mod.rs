mod build_state;
mod builder;
pub mod incremental;
mod service;
mod watcher;

use crate::domain::{Target, TargetId};
use anyhow::{Context, Result};
use build_state::TargetBuildStates;
use builder::build_target;
use crossbeam::channel::Sender;
use crossbeam::thread::Scope;
use incremental::{IncrementalRunResult, IncrementalRunner};
use service::ServicesRunner;
use std::thread::sleep;
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

    pub fn watch<'a, 's>(&'a self, scope: &Scope<'s>) -> Result<()>
    where
        'a: 's,
    {
        let watcher =
            TargetsWatcher::new(&self.targets).with_context(|| "Failed to set up file watcher")?;

        let mut services_runner = ServicesRunner::new(&self.targets);

        let mut target_build_states = TargetBuildStates::new(&self.targets);

        loop {
            let invalidated_builds = watcher
                .get_invalidated_targets()
                .with_context(|| "File watch error")?;
            target_build_states.set_builds_invalidated(&invalidated_builds);

            self.build_ready_targets(scope, &mut target_build_states);

            if let Some(build_report) = target_build_states.get_finished_build()? {
                let target = &self.targets[build_report.target_id];
                if let IncrementalRunResult::Run(Err(e)) = build_report.result {
                    log::warn!("{} - {}", target.name, e);
                } else {
                    services_runner.restart_service(scope, target.clone())?;
                }
            }

            sleep(Duration::from_millis(10))
        }
    }

    pub fn build<'a, 's>(&'a self, scope: &Scope<'s>) -> Result<()>
    where
        'a: 's,
    {
        let mut services_runner = ServicesRunner::new(&self.targets);

        let mut target_build_states = TargetBuildStates::new(&self.targets);

        while !target_build_states.all_are_built() {
            self.build_ready_targets(scope, &mut target_build_states);

            if let Some(build_report) = target_build_states.get_finished_build()? {
                if let IncrementalRunResult::Run(Err(e)) = build_report.result {
                    return Err(e);
                }

                let target = &self.targets[build_report.target_id];
                services_runner.start_service(scope, target.clone())?;
            }

            sleep(Duration::from_millis(10))
        }

        Ok(())
    }

    fn build_ready_targets<'a, 's>(
        &'a self,
        scope: &Scope<'s>,
        target_build_states: &mut TargetBuildStates,
    ) where
        'a: 's,
    {
        for &target_id in &target_build_states.get_ready_to_build_targets() {
            target_build_states.set_build_started(target_id);
            self.build_target(scope, target_id, target_build_states.tx.clone());
        }
    }

    fn build_target<'a, 's>(
        &'a self,
        scope: &Scope<'s>,
        target_id: TargetId,
        tx: Sender<BuildReport>,
    ) where
        'a: 's,
    {
        let target = self.targets.get(target_id).unwrap();
        scope.spawn(move |_| {
            let result = self
                .incremental_runner
                .run(&target, || build_target(&target))
                .with_context(|| format!("{} - Build failed", target.name))
                .unwrap();

            if let IncrementalRunResult::Skipped = result {
                log::info!("{} - Build skipped (Not Modified)", target.name);
            }

            tx.send(BuildReport::new(target.id, result))
                .with_context(|| "Sender error")
                .unwrap();
        });
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
