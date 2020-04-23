use crate::target::{Target, TargetId};
use crossbeam;
use crossbeam::channel::{unbounded, Receiver};
use notify::{RawEvent, RecommendedWatcher, RecursiveMode, Watcher};
use std::env::current_dir;

pub fn setup_watcher(
    targets: &[Target],
) -> Result<(RecommendedWatcher, Receiver<RawEvent>), notify::Error> {
    let (watcher_tx, watcher_rx) = unbounded();
    let mut watcher: RecommendedWatcher = Watcher::new_immediate(watcher_tx)?;
    for target in targets.iter() {
        for watch_path in target.watch_list.iter() {
            watcher.watch(watch_path, RecursiveMode::Recursive)?;
        }
    }

    Ok((watcher, watcher_rx))
}

pub fn raw_event_to_targets(event: RawEvent, targets: &[Target]) -> Result<Vec<TargetId>, String> {
    if let Some(absolute_path) = event.path {
        if let Some(absolute_path) = absolute_path.to_str() {
            let project_dir_path =
                current_dir().map_err(|e| format!("Error getting current dir: {}", e))?;
            let project_dir_as_str = project_dir_path
                .to_str()
                .ok_or_else(|| "Error converting current dir to UTF8")?;

            // TODO: This won't work with symlinks.
            let relative_path = &absolute_path[project_dir_as_str.len() + 1..];

            return Ok(targets
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
