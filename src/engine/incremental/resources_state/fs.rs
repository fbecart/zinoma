use crate::work_dir;
use anyhow::{Context, Result};
use async_std::path::{Path, PathBuf};
use async_std::task;
use seahash::SeaHasher;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::hash::Hasher;
use std::io::{BufReader, Read};
use std::time::{Duration, SystemTime};
use walkdir::WalkDir;

#[derive(Serialize, Deserialize, PartialEq)]
pub struct ResourcesState(HashMap<std::path::PathBuf, (Duration, u64)>);

impl ResourcesState {
    pub async fn current(paths: &[PathBuf]) -> Result<Self> {
        // TODO Here was rayon
        Ok(Self(
            list_files(paths)
                .await
                .into_iter()
                .map(|file| {
                    task::block_on(async {
                        let modified = get_file_modified(&file).await?;
                        let file_hash = compute_file_hash(&file).with_context(|| {
                            format!("Failed to compute hash of {}", file.display())
                        })?;
                        Ok((file.into(), (modified, file_hash)))
                    })
                })
                .collect::<Result<HashMap<_, _>>>()?,
        ))
    }

    pub async fn eq_current_state(&self, paths: &[PathBuf]) -> Result<bool> {
        let files = list_files(paths).await;

        if files.len() != self.0.len() {
            return Ok(false);
        }

        // TODO Here was rayon
        Ok(files.into_iter().all(|file_path| {
            task::block_on(async {
                let std_path: &std::path::Path = file_path.as_path().into();
                match self.0.get(std_path) {
                    None => false,
                    Some(&(saved_modified, saved_hash)) => {
                        match get_file_modified(&file_path).await {
                            Err(e) => {
                                log::error!("{:?}", e);
                                false
                            }
                            Ok(modified) => {
                                modified == saved_modified
                                    || match compute_file_hash(&file_path) {
                                        Err(e) => {
                                            log::error!("{:?}", e);
                                            false
                                        }
                                        Ok(hash) => hash == saved_hash,
                                    }
                            }
                        }
                    }
                }
            })
        }))
    }
}

async fn list_files(paths: &[PathBuf]) -> HashSet<PathBuf> {
    let mut files = HashSet::new();

    for path in paths {
        for entry in WalkDir::new(path).into_iter() {
            match entry {
                Err(e) => log::debug!("Failed to walk dir {}: {}", path.display(), e),
                Ok(entry) => {
                    let path: PathBuf = entry.into_path().into();
                    if path.is_file().await && !work_dir::is_in_work_dir(&path) {
                        files.insert(path);
                    }
                }
            }
        }
    }

    files
}

async fn get_file_modified(file: &Path) -> Result<Duration> {
    let metadata = file
        .metadata()
        .await
        .with_context(|| format!("Failed to obtain metadata of file {}", file.display()))?;
    let modified = metadata.modified().with_context(|| {
        format!(
            "Failed to obtain modified timestamp of file {}",
            file.display()
        )
    })?;
    modified
        .duration_since(SystemTime::UNIX_EPOCH)
        .with_context(|| {
            format!(
                "Failed to obtain duration between UNIX EPOCH and modified timestamp for file {}",
                file.display()
            )
        })
}

fn compute_file_hash(file_path: &Path) -> Result<u64> {
    let mut hasher = SeaHasher::default();
    let file = fs::File::open(file_path)
        .with_context(|| format!("Failed to open file {}", file_path.display()))?;
    let mut reader = BufReader::new(file);

    let mut buffer = [0; 1024];
    loop {
        let count = reader
            .read(&mut buffer)
            .with_context(|| format!("Failed to read file {}", file_path.display()))?;
        if count == 0 {
            break;
        }
        Hasher::write(&mut hasher, &buffer[..count]);
    }

    Ok(hasher.finish())
}
