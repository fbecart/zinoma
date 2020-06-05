use crate::config::yaml;
use crate::work_dir;
use anyhow::{Context, Result};
use rayon::prelude::*;
use seahash::SeaHasher;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::hash::Hasher;
use std::io::{BufReader, Read};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

#[derive(Serialize, Deserialize, PartialEq)]
pub struct ResourcesState(HashMap<PathBuf, u64>);

impl ResourcesState {
    pub fn current(resources: &[yaml::Resource], base_dir: &Path) -> Result<Self> {
        Ok(Self(
            list_files(resources, base_dir)
                .into_par_iter()
                .map(|file| {
                    let file_hash = compute_file_hash(&file)
                        .with_context(|| format!("Failed to compute hash of {}", file.display()))?;
                    Ok((file, file_hash))
                })
                .collect::<Result<HashMap<_, _>>>()?,
        ))
    }

    pub fn eq_current_state(&self, resources: &[yaml::Resource], base_dir: &Path) -> Result<bool> {
        let files = list_files(resources, base_dir);

        if files.len() != self.0.len() {
            return Ok(false);
        }

        Ok(files.par_iter().all(|file_path| {
            match self.0.get(file_path) {
                Some(&saved_hash) => compute_file_hash(file_path)
                    .map(|hash| hash == saved_hash)
                    .unwrap_or_else(|e| {
                        log::error!("{:?}", e);
                        false // Propagating the error would be better, but I don't know how this can be achieved
                    }),
                None => false,
            }
        }))
    }
}

fn list_files(resources: &[yaml::Resource], base_dir: &Path) -> HashSet<PathBuf> {
    let mut files = HashSet::new();

    for resource in resources {
        if let yaml::Resource::Paths { paths } = resource {
            for path in paths {
                for entry in WalkDir::new(base_dir.join(path)) {
                    match entry {
                        Err(e) => log::debug!("Failed to walk dir {}: {}", path, e),
                        Ok(entry) => {
                            let path = entry.path().to_path_buf();
                            if path.is_file() && !work_dir::is_in_work_dir(&path) {
                                files.insert(path);
                            }
                        }
                    }
                }
            }
        }
    }

    files
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
