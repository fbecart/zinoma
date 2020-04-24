use crate::target::{Target, TargetId};
use crossbeam::channel::{unbounded, Receiver, TryRecvError};
use notify::{RawEvent, RecommendedWatcher, RecursiveMode, Watcher};

pub struct TargetsWatcher {
    target_watchers: Vec<TargetWatcher>,
}

impl TargetsWatcher {
    pub fn new(targets: &[Target]) -> Result<Self, String> {
        let mut target_watchers = Vec::new();
        for target in targets.iter() {
            target_watchers.push(TargetWatcher::new(target)?);
        }
        Ok(Self { target_watchers })
    }

    pub fn get_invalidated_targets(&self) -> Result<Vec<TargetId>, String> {
        let mut invalidated_targets = Vec::new();

        for (target_id, target_watcher) in self.target_watchers.iter().enumerate() {
            if target_watcher.is_invalidated()? {
                invalidated_targets.push(target_id);
            }
        }

        Ok(invalidated_targets)
    }
}

pub struct TargetWatcher {
    rx: Receiver<RawEvent>,
    _watcher: RecommendedWatcher,
}

impl TargetWatcher {
    pub fn new(target: &Target) -> Result<Self, String> {
        let (tx, rx) = unbounded();
        let mut watcher: RecommendedWatcher =
            Watcher::new_immediate(tx).map_err(|e| format!("Error creating watcher: {}", e))?;

        for watch_path in target.watch_list.iter() {
            watcher
                .watch(watch_path, RecursiveMode::Recursive)
                .map_err(|e| {
                    format!(
                        "Error watching path {} for target {}: {}",
                        watch_path.display(),
                        target.name,
                        e
                    )
                })?;
        }

        Ok(Self {
            rx,
            _watcher: watcher,
        })
    }

    pub fn is_invalidated(&self) -> Result<bool, String> {
        match self.rx.try_recv() {
            Ok(_) => Ok(true),
            Err(TryRecvError::Empty) => Ok(false),
            Err(e) => Err(format!("Watcher received error: {}", e)),
        }
    }
}
