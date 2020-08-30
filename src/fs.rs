use crate::domain::{self, FilesResource};
use crate::work_dir;
use async_std::path::{Path, PathBuf};
use async_std::task;
use domain::FileExtensions;
use futures::future;
use std::collections::HashSet;
use std::sync::Arc;
use walkdir::WalkDir;
use work_dir::is_work_dir;

pub async fn list_files_in_resources(resources: &[FilesResource]) -> HashSet<PathBuf> {
    future::join_all(
        resources
            .iter()
            .map(|resource| list_files_in_paths(&resource.paths, &resource.extensions)),
    )
    .await
    .into_iter()
    .flatten()
    .collect()
}

pub async fn list_files_in_paths(
    paths: &[PathBuf],
    extensions: &FileExtensions,
) -> HashSet<PathBuf> {
    future::join_all(
        paths
            .iter()
            .map(|path| list_files_in_path(path, extensions)),
    )
    .await
    .into_iter()
    .flatten()
    .collect()
}

async fn list_files_in_path(path: &Path, extensions: &FileExtensions) -> Vec<PathBuf> {
    let walkdir = WalkDir::new(path);

    let extensions = Arc::from(extensions.clone());
    task::spawn_blocking(move || {
        walkdir
            .into_iter()
            .filter_entry(|e| !is_work_dir(e))
            .filter_map(|entry| match entry {
                Err(e) => {
                    log::debug!("Failed to walk dir: {}", e);
                    None
                }
                Ok(entry) => {
                    let path = entry.into_path();
                    Some(path)
                        .filter(|path| path.is_file())
                        .filter(|file| domain::matches_extensions(file, extensions.as_ref()))
                        .map(|path| path.into())
                }
            })
            .collect()
    })
    .await
}
