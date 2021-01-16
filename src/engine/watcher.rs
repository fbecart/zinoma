use crate::domain::{self, Resources, TargetId};
use crate::work_dir;
use anyhow::{Context, Error, Result};
use async_std::channel::Sender;
use async_std::path::{Path, PathBuf};
use domain::FileExtensions;
use notify::{ErrorKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::collections::{HashMap, HashSet};

pub struct TargetWatcher {
    _watchers: Vec<RecommendedWatcher>,
}

impl TargetWatcher {
    pub fn new(
        target_id: &TargetId,
        target_input: Option<&Resources>,
        target_invalidated_sender: &Sender<TargetInvalidatedMessage>,
    ) -> Result<Option<Self>> {
        if let Some(target_input) = target_input {
            let mut paths_grouped_by_extensions = HashMap::new();
            for resource in &target_input.files {
                paths_grouped_by_extensions
                    .entry(resource.extensions.clone())
                    .or_insert_with(HashSet::new)
                    .extend(resource.paths.iter());
            }

            let watchers = paths_grouped_by_extensions
                .into_iter()
                .filter(|(_extensions, paths)| !paths.is_empty())
                .map(|(extensions, paths)| {
                    let mut watcher = Self::build_immediate_watcher(
                        target_id.clone(),
                        target_invalidated_sender.clone(),
                        extensions,
                    )?;

                    for path in paths {
                        match watcher.watch(path, RecursiveMode::Recursive) {
                            Ok(_) => {}
                            Err(notify::Error {
                                kind: ErrorKind::PathNotFound,
                                ..
                            }) => {
                                log::warn!(
                                    "{} - Skipping watch on non-existing path: {}",
                                    target_id,
                                    path.display(),
                                );
                            }
                            Err(e) => {
                                return Err(Error::new(e).context(format!(
                                    "Error watching path {} for target {}",
                                    path.display(),
                                    target_id,
                                )));
                            }
                        }
                    }

                    Ok(watcher)
                })
                .collect::<Result<Vec<_>>>()?;

            return Ok(Some(Self {
                _watchers: watchers,
            }));
        }

        Ok(None)
    }

    fn build_immediate_watcher(
        target_id: TargetId,
        target_invalidated_sender: Sender<TargetInvalidatedMessage>,
        extensions: FileExtensions,
    ) -> Result<RecommendedWatcher> {
        Watcher::new_immediate(move |result: notify::Result<notify::Event>| {
            let relevant_files = result
                .unwrap()
                .paths
                .into_iter()
                .filter(|path| {
                    let path: PathBuf = path.into();
                    !is_tmp_editor_file(&path)
                        && !work_dir::is_in_work_dir(&path)
                        && domain::matches_extensions(path.as_path().into(), &extensions)
                })
                .collect::<Vec<_>>();

            if !relevant_files.is_empty() {
                let target_id = target_id.clone();
                log::trace!(
                    "{} - Invalidated by {}",
                    &target_id,
                    itertools::join(relevant_files.iter().flat_map(|path| path.to_str()), ", ")
                );
                if target_invalidated_sender
                    .try_send(TargetInvalidatedMessage)
                    .is_err()
                {
                    log::trace!("{} - Target already invalidated. Skipping.", &target_id);
                }
            }
        })
        .with_context(|| "Error creating watcher")
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

pub struct TargetInvalidatedMessage;

#[cfg(test)]
mod is_tmp_editor_file_tests {
    use super::is_tmp_editor_file;
    use async_std::path::Path;

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
