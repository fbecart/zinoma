use anyhow::{Context, Result};
use rayon::prelude::*;
use seahash::SeaHasher;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::hash::Hasher;
use std::io::{BufReader, Read};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

pub fn compute_file_hashes_in_paths(paths: &[PathBuf]) -> Result<HashMap<PathBuf, u64>> {
    let files = list_files(paths).with_context(|| "Failed to list checksum files".to_string())?;

    files
        .into_par_iter()
        .map(|file| {
            let file_hash = compute_file_hash(&file)
                .with_context(|| format!("Failed to compute hash of {}", file.display()))?;
            Ok((file, file_hash))
        })
        .collect::<Result<HashMap<_, _>>>()
}

fn list_files(paths: &[PathBuf]) -> Result<HashSet<PathBuf>> {
    let mut files = HashSet::new();

    for path in paths {
        for entry in WalkDir::new(path) {
            let path = entry
                .with_context(|| format!("Failed to traverse directory {}", path.display()))?
                .path()
                .to_path_buf();
            if path.is_file() {
                files.insert(path);
            }
        }
    }

    Ok(files)
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
