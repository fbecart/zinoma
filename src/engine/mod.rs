mod build_state;
mod builder;
mod service;
mod watcher;

use crate::incremental::{IncrementalRunResult, IncrementalRunner};
use crate::target::Target;
use build_state::TargetBuildStates;
use builder::TargetBuilder;
use crossbeam::thread::Scope;
use service::ServicesRunner;
use std::thread::sleep;
use std::time::Duration;
use watcher::TargetsWatcher;

pub struct Engine<'a> {
    targets: Vec<Target>,
    target_builder: TargetBuilder<'a>,
}

impl<'a> Engine<'a> {
    pub fn new(targets: Vec<Target>, incremental_runner: IncrementalRunner<'a>) -> Self {
        Self {
            targets,
            target_builder: TargetBuilder::new(incremental_runner),
        }
    }

    pub fn watch(&'a self, scope: &Scope<'a>) -> Result<(), String> {
        let watcher = TargetsWatcher::new(&self.targets)
            .map_err(|e| format!("Failed to set up file watcher: {}", e))?;

        let mut services_runner = ServicesRunner::new(&self.targets);

        let mut target_build_states = TargetBuildStates::new(&self.targets);

        loop {
            let invalidated_builds = watcher
                .get_invalidated_targets()
                .map_err(|e| format!("File watch error: {}", e))?;
            target_build_states.set_builds_invalidated(&invalidated_builds);

            self.build_ready_targets(scope, &mut target_build_states);

            if let Some(result) = target_build_states.get_finished_build()? {
                let target = &self.targets[result.target_id];
                if let IncrementalRunResult::Run(Err(e)) = result.result {
                    log::warn!("{} - Build failed: {}", target.name, e);
                } else {
                    services_runner.restart_service(scope, target)?;
                }
            }

            sleep(Duration::from_millis(10))
        }
    }

    pub fn build(&'a self, scope: &Scope<'a>) -> Result<(), String> {
        let mut target_build_states = TargetBuildStates::new(&self.targets);

        while !target_build_states.all_are_built() {
            self.build_ready_targets(scope, &mut target_build_states);

            if let Some(result) = target_build_states.get_finished_build()? {
                if let IncrementalRunResult::Run(Err(e)) = result.result {
                    let target = &self.targets[result.target_id];
                    return Err(format!("Build failed for target {}: {}", target.name, e));
                }
            }

            sleep(Duration::from_millis(10))
        }

        Ok(())
    }

    fn build_ready_targets(
        &'a self,
        scope: &Scope<'a>,
        target_build_states: &mut TargetBuildStates,
    ) {
        for &target_id in target_build_states.get_ready_to_build_targets().iter() {
            let target = self.targets.get(target_id).unwrap();
            target_build_states.set_build_started(target.id);
            self.target_builder
                .build(scope, target, &target_build_states.tx);
        }
    }
}
