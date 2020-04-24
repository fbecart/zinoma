use crate::target::{Target, TargetId};
use crossbeam::channel::{unbounded, Receiver, TryRecvError};
use notify::{RawEvent, RecommendedWatcher, RecursiveMode, Watcher};
use std::env::current_dir;

pub struct TargetWatcher {
    rx: Receiver<RawEvent>,
    _watcher: RecommendedWatcher,
    targets: Vec<Target>,
}

impl TargetWatcher {
    pub fn new(targets: &[Target]) -> Result<Self, String> {
        let (tx, rx) = unbounded();
        let watcher =
            Watcher::new_immediate(tx).map_err(|e| format!("Error creating watcher: {}", e))?;

        let mut myself = Self {
            rx,
            _watcher: watcher,
            targets: Vec::new(),
        };

        for target in targets {
            myself.watch_target(target)?;
        }

        Ok(myself)
    }

    fn watch_target(&mut self, target: &Target) -> Result<(), String> {
        for watch_path in target.watch_list.iter() {
            self._watcher
                .watch(watch_path, RecursiveMode::Recursive)
                .map_err(|e| {
                    format!(
                        "Error watching path {} for target {}: {}",
                        watch_path, target.name, e
                    )
                })?;
        }

        self.targets.push(target.clone());

        Ok(())
    }

    pub fn get_invalidated_targets(&self) -> Result<Vec<TargetId>, String> {
        match self.rx.try_recv() {
            Ok(event) => self.raw_event_to_targets(event),
            Err(TryRecvError::Empty) => Ok(Vec::new()),
            Err(e) => Err(format!("Watcher received error: {}", e)),
        }
    }

    fn raw_event_to_targets(&self, event: RawEvent) -> Result<Vec<TargetId>, String> {
        if let Some(absolute_path) = event.path {
            if let Some(absolute_path) = absolute_path.to_str() {
                let project_dir_path =
                    current_dir().map_err(|e| format!("Error getting current dir: {}", e))?;
                let project_dir_as_str = project_dir_path
                    .to_str()
                    .ok_or_else(|| "Error converting current dir to UTF8")?;

                // TODO: This won't work with symlinks.
                let relative_path = &absolute_path[project_dir_as_str.len() + 1..];

                return Ok(self
                    .targets
                    .iter()
                    .filter(|target| {
                        target
                            .watch_list
                            .iter()
                            .any(|watch_path| relative_path.starts_with(watch_path))
                    })
                    .map(|target| target.id)
                    .collect());
            }
        }

        Ok(Vec::new())
    }
}
