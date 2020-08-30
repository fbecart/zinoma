use crate::async_utils::all;
use crate::domain::FilesResource;
use anyhow::{Context, Result};
use async_std::fs::File;
use async_std::io::BufReader;
use async_std::path::Path;
use async_std::prelude::*;
use seahash::SeaHasher;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::hash::Hasher;
use std::time::{Duration, SystemTime};

#[derive(Serialize, Deserialize, PartialEq)]
pub struct ResourcesState(HashMap<std::path::PathBuf, (Duration, u64)>);

impl ResourcesState {
    pub async fn current(resources: &[FilesResource]) -> Result<Self> {
        let files = crate::fs::list_files_in_resources(resources).await;

        let mut state = HashMap::with_capacity(files.len());

        for file in files {
            let modified = get_file_modified(&file).await.with_context(|| {
                format!("Failed to obtain file modified date: {}", file.display())
            })?;
            let file_hash = compute_file_hash(&file)
                .await
                .with_context(|| format!("Failed to compute hash of {}", file.display()))?;
            state.insert(file.into(), (modified, file_hash));
        }

        Ok(Self(state))
    }

    pub async fn eq_current_state(&self, resources: &[FilesResource]) -> bool {
        let files = crate::fs::list_files_in_resources(resources).await;

        if files.len() != self.0.len() {
            return false;
        }

        let futures = files.into_iter().map(|file_path| async move {
            let std_path: &std::path::Path = file_path.as_path().into();
            match self.0.get(std_path) {
                None => false,
                Some(&(saved_modified, saved_hash)) => match get_file_modified(&file_path).await {
                    Err(e) => {
                        log::error!("{:?}", e);
                        false
                    }
                    Ok(modified) => {
                        modified == saved_modified
                            || match compute_file_hash(&file_path).await {
                                Err(e) => {
                                    log::error!("{:?}", e);
                                    false
                                }
                                Ok(hash) => hash == saved_hash,
                            }
                    }
                },
            }
        });

        all(futures).await
    }
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

async fn compute_file_hash(file_path: &Path) -> Result<u64> {
    let mut hasher = SeaHasher::default();
    let file = File::open(file_path)
        .await
        .with_context(|| format!("Failed to open file {}", file_path.display()))?;
    let mut reader = BufReader::new(file);

    let mut buffer = [0; 1024];
    loop {
        let count = reader
            .read(&mut buffer)
            .await
            .with_context(|| format!("Failed to read file {}", file_path.display()))?;
        if count == 0 {
            break;
        }
        Hasher::write(&mut hasher, &buffer[..count]);
    }

    Ok(hasher.finish())
}
