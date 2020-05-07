use crate::domain::Target;
use anyhow::{Context, Error, Result};
use seahash::SeaHasher;
use serde::{Deserialize, Serialize};
use std::fs;
use std::hash::Hasher;
use std::io::ErrorKind;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::Instant;
use walkdir::WalkDir;

#[derive(PartialEq)]
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

    pub fn run<T, F>(&self, target: &Target, function: F) -> Result<IncrementalRunResult<Result<T>>>
    where
        F: Fn() -> Result<T>,
    {
        let saved_checksums = self.read_target_checksums(target)?;
        let target_checksums = compute_target_checksums(target)?;
        if saved_checksums.is_some() && saved_checksums == target_checksums {
            return Ok(IncrementalRunResult::Skipped);
        };

        self.remove_target_checksums(&target)?;

        let result = function();

        if result.is_ok() {
            if let Some(target_checksums) = target_checksums {
                let target_checksums = TargetChecksums {
                    outputs: compute_checksum(&target.name, &target.output_paths)?,
                    ..target_checksums
                };
                self.write_target_checksums(&target, &target_checksums)?;
            }
        }

        Ok(IncrementalRunResult::Run(result))
    }

    fn get_checksum_file(&self, target: &Target) -> PathBuf {
        self.checksum_dir
            .join(format!("{}.checksum.json", target.name))
    }

    fn read_target_checksums(&self, target: &Target) -> Result<Option<TargetChecksums>> {
        // Might want to check for some errors like permission denied.
        fs::create_dir(&self.checksum_dir).ok();

        let checksum_file = self.get_checksum_file(target);
        match fs::read_to_string(&checksum_file) {
            Ok(file_content) => match serde_json::from_str(&file_content) {
                Ok(checksums) => Ok(Some(checksums)),
                Err(e) => {
                    log::trace!(
                        "{} - Dropping corrupted checksum file (Error: {})",
                        &target.name,
                        e
                    );
                    self.remove_target_checksums(&target)?;
                    Ok(None)
                }
            },
            Err(e) if e.kind() == ErrorKind::NotFound => Ok(None),
            Err(e) => Err(Error::new(e).context(format!(
                "Failed reading checksum file {} for target {}",
                checksum_file.display(),
                &target.name
            ))),
        }
    }

    fn remove_target_checksums(&self, target: &Target) -> Result<()> {
        let checksum_file = &self.get_checksum_file(target);
        if checksum_file.exists() {
            fs::remove_file(&checksum_file).with_context(|| {
                format!(
                    "Failed to delete checksum file {} for target {}",
                    checksum_file.display(),
                    target.name
                )
            })?;
        }
        Ok(())
    }

    fn write_target_checksums(&self, target: &Target, checksums: &TargetChecksums) -> Result<()> {
        let checksum_file = self.get_checksum_file(target);
        let mut file = fs::File::create(&checksum_file).with_context(|| {
            format!(
                "Failed to create checksum file {} for target {}",
                checksum_file.display(),
                target.name
            )
        })?;
        let file_content = serde_json::to_string(checksums)
            .with_context(|| format!("Failed to serialize checksums for {}", target.name))?;
        file.write_all(file_content.as_bytes()).with_context(|| {
            format!(
                "Failed to write checksum file {} for target {}",
                checksum_file.display(),
                target.name
            )
        })
    }

    pub fn clean_checksums(&self, targets: &[Target]) -> Result<()> {
        if targets.is_empty() {
            self.remove_checksum_dir()
        } else {
            for target in targets.iter() {
                self.remove_target_checksums(target)?;
            }
            Ok(())
        }
    }

    pub fn remove_checksum_dir(&self) -> Result<()> {
        match std::fs::remove_dir_all(self.checksum_dir) {
            Ok(_) => {}
            Err(e) if e.kind() == ErrorKind::NotFound => {}
            Err(e) => {
                return Err(Error::new(e).context(format!(
                    "Failed to remove checksum directory {}",
                    self.checksum_dir.display()
                )));
            }
        }

        Ok(())
    }
}

fn compute_target_checksums(target: &Target) -> Result<Option<TargetChecksums>> {
    if target.input_paths.is_empty() {
        Ok(None)
    } else {
        Ok(Some(TargetChecksums {
            inputs: compute_checksum(&target.name, &target.input_paths)?,
            outputs: compute_checksum(&target.name, &target.output_paths)?,
        }))
    }
}

fn compute_checksum(identifier: &str, paths: &[PathBuf]) -> Result<Option<u64>> {
    if paths.is_empty() {
        return Ok(None);
    }

    let computation_start = Instant::now();
    log::trace!("{} - Computing checksum", identifier);
    let mut hasher = SeaHasher::default();

    for path in paths.iter() {
        let checksum = calculate_path_checksum(path)?;
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

fn calculate_path_checksum(path: &Path) -> Result<Option<u64>> {
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

#[derive(Serialize, Deserialize, PartialEq)]
struct TargetChecksums {
    inputs: Option<u64>,
    outputs: Option<u64>,
}
