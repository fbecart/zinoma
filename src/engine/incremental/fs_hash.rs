use anyhow::{Context, Result};
use seahash::SeaHasher;
use std::fs;
use std::hash::Hasher;
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
        let checksum = calculate_path_hash(path)?;
        Hasher::write_u64(&mut hasher, checksum.unwrap_or(0));
    }

    let computation_duration = computation_start.elapsed();
    log::trace!(
        "{} - Checksum computed (took {}ms)",
        identifier,
        computation_duration.as_millis()
    );

    Ok(Some(hasher.finish()))
}

fn calculate_path_hash(path: &Path) -> Result<Option<u64>> {
    if !path.exists() {
        return Ok(None);
    }

    let mut hasher = SeaHasher::default();

    for entry in WalkDir::new(path) {
        let entry = entry.with_context(|| "Failed to traverse directory")?;

        if entry.path().is_file() {
            let contents = fs::read(entry.path())
                .with_context(|| "Failed to read file to calculate checksum")?;
            Hasher::write(&mut hasher, &contents);
        }
    }

    Ok(Some(hasher.finish()))
}
