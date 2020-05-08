use anyhow::{Context, Result};
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
    let mut hasher = SeaHasher::default();

    for path in paths.iter() {
        let path_hash = compute_path_hash(path)?;
        Hasher::write_u64(&mut hasher, path_hash.unwrap_or(0));
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

    let mut hasher = SeaHasher::default();

    for entry in WalkDir::new(path) {
        let entry = entry.with_context(|| "Failed to traverse directory")?;
        let path = entry.path();
        if path.is_file() {
            let file_hash = compute_file_hash(path)
                .with_context(|| format!("Failed to compute hash of file {}", path.display()))?;
            Hasher::write_u64(&mut hasher, file_hash);
        }
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
