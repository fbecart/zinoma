use crate::domain::{Target, TargetId};
use anyhow::{Context, Error, Result};
use crossbeam::channel::{unbounded, Receiver, TryRecvError};
use notify::{ErrorKind, Event, FsEventWatcher, RecursiveMode, Watcher};
use std::path::{Path, PathBuf};

pub struct TargetsWatcher<'a> {
    target_watchers: Vec<TargetWatcher<'a>>,
}

impl<'a> TargetsWatcher<'a> {
    pub fn new(targets: &'a [Target]) -> Result<Self> {
        let mut target_watchers = Vec::new();
        for target in targets.iter() {
            target_watchers.push(TargetWatcher::new(target)?);
        }
        Ok(Self { target_watchers })
    }

    pub fn get_invalidated_targets(&self) -> Result<Vec<TargetId>> {
        let mut invalidated_targets = Vec::new();

        for (target_id, target_watcher) in self.target_watchers.iter().enumerate() {
            if target_watcher.is_invalidated()? {
                invalidated_targets.push(target_id);
            }
        }

        Ok(invalidated_targets)
    }
}

pub struct TargetWatcher<'a> {
    target: &'a Target,
    rx: Receiver<notify::Result<Event>>,
    _watcher: FsEventWatcher,
}

impl<'a> TargetWatcher<'a> {
    pub fn new(target: &'a Target) -> Result<Self> {
        let (tx, rx) = unbounded();
        let mut watcher: FsEventWatcher =
            Watcher::new_immediate(move |e| tx.send(e).with_context(|| "Sender error").unwrap())
                .with_context(|| "Error creating watcher")?;

        for path in target.input_paths.iter() {
            match watcher.watch(path, RecursiveMode::Recursive) {
                Ok(_) => {}
                Err(notify::Error {
                    kind: ErrorKind::PathNotFound,
                    ..
                }) => {
                    log::warn!(
                        "{} - Skipping watch on non-existing path: {}",
                        target.name,
                        path.display(),
                    );
                }
                Err(e) => {
                    return Err(Error::new(e).context(format!(
                        "Error watching path {} for target {}",
                        path.display(),
                        target.name,
                    )));
                }
            }
        }

        Ok(Self {
            target,
            rx,
            _watcher: watcher,
        })
    }

    pub fn is_invalidated(&self) -> Result<bool> {
        match self.rx.try_recv() {
            Ok(event) => {
                let paths: Vec<PathBuf> = event
                    .unwrap()
                    .paths
                    .into_iter()
                    .filter(|path| !is_tmp_editor_file(path))
                    .collect();

                let invalidated = !paths.is_empty();
                if invalidated {
                    log::trace!("{} - Invalidated by {:?}", self.target.name, paths)
                }

                Ok(invalidated)
            }
            Err(TryRecvError::Empty) => Ok(false),
            Err(e) => Err(Error::new(e).context("Watcher received error")),
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
