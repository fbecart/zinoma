use crate::target::{Target, TargetId};
use crossbeam::channel::{unbounded, Receiver, TryRecvError};
use notify::{RawEvent, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::Path;

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
            Ok(event) => {
                let path = event.path.unwrap();
                Ok(!is_tmp_editor_file(&path))
            }
            Err(TryRecvError::Empty) => Ok(false),
            Err(e) => Err(format!("Watcher received error: {}", e)),
        }
    }
}

fn is_tmp_editor_file(file_path: &Path) -> bool {
    let file_name = file_path.file_name().unwrap();
    let file_name = file_name.to_str().unwrap();

    if file_name.ends_with('~') {
        return true; // IntelliJ IDEA
    }

    if file_name.starts_with('.') && (file_name.ends_with(".swp") || file_name.ends_with(".swx")) {
        return true; // Vim
    }

    false
}

#[cfg(test)]
mod is_tmp_editor_file_tests {
    use super::is_tmp_editor_file;
    use std::path::Path;

    #[test]
    fn src_file_should_not_be_tmp_editor_file() {
        assert!(!is_tmp_editor_file(Path::new("/my/project/src/main.rs")));
    }

    #[test]
    fn intellij_tmp_file_should_be_tmp_editor_file() {
        assert!(is_tmp_editor_file(Path::new("/my/project/src/main.rs~")));
    }

    #[test]
    fn vim_tmp_file_should_be_tmp_editor_file() {
        let path = Path::new("/my/project/src/.main.rs.swp");
        assert!(is_tmp_editor_file(path));

        let path = Path::new("/my/project/src/.main.rs.swx");
        assert!(is_tmp_editor_file(path));
    }
}
