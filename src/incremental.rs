use crypto::digest::Digest;
use crypto::sha1::Sha1;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::Instant;
use walkdir::WalkDir;

pub enum IncrementalRunResult<T> {
    Skipped,
    Run(T),
}

pub struct IncrementalRunner<'a> {
    checksum_dir: &'a Path,
}

impl<'a> IncrementalRunner<'a> {
    pub fn new(checksum_dir: &'a Path) -> Self {
        Self { checksum_dir }
    }

    pub fn run<T, E, F>(
        &self,
        identifier: &str,
        input_files: &[PathBuf],
        function: F,
    ) -> Result<IncrementalRunResult<Result<T, E>>, String>
    where
        F: Fn() -> Result<T, E>,
    {
        let watch_checksum = if input_files.is_empty() {
            None
        } else {
            let computation_start = Instant::now();
            log::trace!("{} - Computing checksum", identifier);
            let mut hasher = Sha1::new();

            for path in input_files.iter() {
                let checksum = calculate_checksum(path)?;
                hasher.input_str(&checksum);
            }

            let computation_duration = computation_start.elapsed();
            log::trace!(
                "{} - Checksum computed (took {}ms)",
                identifier,
                computation_duration.as_millis()
            );
            Some(hasher.result_str())
        };

        if let Some(watch_checksum) = &watch_checksum {
            if self.does_checksum_match(identifier, &watch_checksum)? {
                return Ok(IncrementalRunResult::Skipped);
            }
        }

        self.reset_checksum(identifier)?;

        let result = function();

        if result.is_ok() {
            if let Some(watch_checksum) = watch_checksum {
                self.write_checksum(identifier, &watch_checksum)?;
            }
        }

        Ok(IncrementalRunResult::Run(result))
    }

    fn get_checksum_file(&self, target: &str) -> PathBuf {
        self.checksum_dir.join(format!("{}.checksum", target))
    }

    fn does_checksum_match(&self, target: &str, checksum: &str) -> Result<bool, String> {
        // Might want to check for some errors like permission denied.
        fs::create_dir(&self.checksum_dir).ok();
        let checksum_file = self.get_checksum_file(target);
        match fs::read_to_string(&checksum_file) {
            Ok(old_checksum) => Ok(*checksum == old_checksum),
            Err(e) => {
                if e.kind() == std::io::ErrorKind::NotFound {
                    // No checksum found.
                    Ok(false)
                } else {
                    Err(format!(
                        "Failed reading checksum file {} for target {}: {}",
                        checksum_file.display(),
                        target,
                        e
                    ))
                }
            }
        }
    }

    fn reset_checksum(&self, target: &str) -> Result<(), String> {
        let checksum_file = &self.get_checksum_file(target);
        if checksum_file.exists() {
            fs::remove_file(&checksum_file).map_err(|_| {
                format!(
                    "Failed to delete checksum file {} for target {}",
                    checksum_file.display(),
                    target
                )
            })?;
        }
        Ok(())
    }

    fn write_checksum(&self, target: &str, checksum: &str) -> Result<(), String> {
        let checksum_file = self.get_checksum_file(target);
        let mut file = fs::File::create(&checksum_file).map_err(|_| {
            format!(
                "Failed to create checksum file {} for target {}",
                checksum_file.display(),
                target
            )
        })?;
        file.write_all(checksum.as_bytes()).map_err(|_| {
            format!(
                "Failed to write checksum file {} for target {}",
                checksum_file.display(),
                target
            )
        })?;
        Ok(())
    }
}

fn calculate_checksum(path: &Path) -> Result<String, String> {
    let mut hasher = Sha1::new();

    for entry in WalkDir::new(path) {
        let entry = entry.map_err(|e| format!("Failed to traverse directory: {}", e))?;

        if entry.path().is_file() {
            let entry_path = match entry.path().to_str() {
                Some(s) => s,
                None => return Err("Failed to convert file path into String".to_owned()),
            };
            let contents = fs::read(entry_path)
                .map_err(|e| format!("Failed to read file to calculate checksum: {}", e))?;
            hasher.input(contents.as_slice());
        }
    }

    Ok(hasher.result_str())
}
