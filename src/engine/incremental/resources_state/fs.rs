use crate::work_dir;
use anyhow::{Context, Result};
use seahash::SeaHasher;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::hash::Hasher;
use std::io::{BufReader, Read};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};
use walkdir::WalkDir;

#[derive(Serialize, Deserialize, PartialEq)]
pub struct ResourcesState(HashMap<PathBuf, (Duration, u64)>);

impl ResourcesState {
    pub fn current(paths: &[PathBuf]) -> Result<Self> {
        // TODO Here was rayon
        Ok(Self(
            list_files(paths)
                .into_iter()
                .map(|file| {
                    let modified = get_file_modified(&file)?;
                    let file_hash = compute_file_hash(&file)
                        .with_context(|| format!("Failed to compute hash of {}", file.display()))?;
                    Ok((file, (modified, file_hash)))
                })
                .collect::<Result<HashMap<_, _>>>()?,
        ))
    }

    pub fn eq_current_state(&self, paths: &[PathBuf]) -> Result<bool> {
        let files = list_files(paths);

        if files.len() != self.0.len() {
            return Ok(false);
        }

        // TODO Here was rayon
        Ok(files.iter().all(|file_path| match self.0.get(file_path) {
            None => false,
            Some(&(saved_modified, saved_hash)) => match get_file_modified(&file_path) {
                Err(e) => {
                    log::error!("{:?}", e);
                    false
                }
                Ok(modified) => {
                    modified == saved_modified
                        || match compute_file_hash(file_path) {
                            Err(e) => {
                                log::error!("{:?}", e);
                                false
                            }
                            Ok(hash) => hash == saved_hash,
                        }
                }
            },
        }))
    }
}

fn list_files(paths: &[PathBuf]) -> HashSet<PathBuf> {
    let mut files = HashSet::new();

    for path in paths {
        for entry in WalkDir::new(path).into_iter() {
            match entry {
                Err(e) => log::debug!("Failed to walk dir {}: {}", path.display(), e),
                Ok(entry) => {
                    let path = entry.into_path();
                    if path.is_file() && !work_dir::is_in_work_dir(&path) {
                        files.insert(path);
                    }
                }
            }
        }
    }

    files
}

fn get_file_modified(file: &Path) -> Result<Duration> {
    let metadata = file
        .metadata()
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
