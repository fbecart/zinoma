use anyhow::{Context, Result};
use rayon::prelude::*;
use seahash::SeaHasher;
use std::fs;
use std::hash::Hasher;
use std::io::{BufReader, Read};
use std::path::{Path, PathBuf};
use std::time::Instant;
use walkdir::WalkDir;

pub fn compute_paths_hash(identifier: &str, paths: &[PathBuf]) -> Result<Option<u64>> {
    if paths.is_empty() {
        return Ok(None);
    }

    let computation_start = Instant::now();
    log::trace!("{} - Computing checksum", identifier);

    let path_hashes = paths
        .par_iter()
        .map(|path| compute_path_hash(path))
        .collect::<Result<Vec<_>>>()?;

    let mut hasher = SeaHasher::default();
    for hash in path_hashes {
        Hasher::write_u64(&mut hasher, hash.unwrap_or(0));
    }

    let computation_duration = computation_start.elapsed();
    log::trace!(
        "{} - Checksum computed (took {}ms)",
        identifier,
        computation_duration.as_millis()
    );

    Ok(Some(hasher.finish()))
}

fn compute_path_hash(path: &Path) -> Result<Option<u64>> {
    if !path.exists() {
        return Ok(None);
    }

    let files = WalkDir::new(path)
        .into_iter()
        .map(|result| result.with_context(|| "Failed to traverse directory"))
        .collect::<Result<Vec<_>>>()?;

    let file_hashes = files
        .into_par_iter()
        .filter(|entry| entry.path().is_file())
        .map(|entry| {
            compute_file_hash(entry.path()).with_context(|| {
                format!("Failed to compute hash of file {}", entry.path().display())
            })
        })
        .collect::<Result<Vec<_>>>()?;

    let mut hasher = SeaHasher::default();
    for hash in file_hashes {
        Hasher::write_u64(&mut hasher, hash);
    }

    Ok(Some(hasher.finish()))
}

fn compute_file_hash(file_path: &Path) -> Result<u64> {
    let mut hasher = SeaHasher::default();
    let file = fs::File::open(file_path)
        .with_context(|| format!("Failed to open file {}", file_path.display()))?;
    let mut reader = BufReader::new(file);

    let mut buffer = [0; 1024];
    loop {
        let count = reader.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        Hasher::write(&mut hasher, &buffer[..count]);
    }

    Ok(hasher.finish())
}
