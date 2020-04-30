use crate::target::Target;
use anyhow::{Context, Error, Result};
use fasthash::XXHasher;
use std::fs;
use std::hash::{Hash, Hasher};
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
        let inputs_identifier = format!("{}.inputs", target.name);
        let outputs_identifier = format!("{}.outputs", target.name);

        let inputs_checksum = compute_checksum(&inputs_identifier, &target.input_paths)?;
        let outputs_checksum = compute_checksum(&outputs_identifier, &target.output_paths)?;

        if self.does_checksum_match(&inputs_identifier, &inputs_checksum)?
            && self.does_checksum_match(&outputs_identifier, &outputs_checksum)?
        {
            return Ok(IncrementalRunResult::Skipped);
        }

        self.erase_checksum(&inputs_identifier)?;
        self.erase_checksum(&outputs_identifier)?;

        let result = function();

        if result.is_ok() {
            self.write_checksum(&inputs_identifier, &inputs_checksum)?;

            let outputs_checksum = compute_checksum(&outputs_identifier, &target.output_paths)?;
            self.write_checksum(&outputs_identifier, &outputs_checksum)?;
        }

        Ok(IncrementalRunResult::Run(result))
    }

    fn get_checksum_file(&self, identifier: &str) -> PathBuf {
        self.checksum_dir.join(format!("{}.checksum", identifier))
    }

    fn does_checksum_match(&self, identifier: &str, checksum: &Option<String>) -> Result<bool> {
        let saved_checksum = self.read_checksum(identifier)?;
        Ok(checksum == &saved_checksum)
    }

    fn read_checksum(&self, identifier: &str) -> Result<Option<String>> {
        // Might want to check for some errors like permission denied.
        fs::create_dir(&self.checksum_dir).ok();
        let checksum_file = self.get_checksum_file(identifier);
        match fs::read_to_string(&checksum_file) {
            Ok(checksum) => Ok(Some(checksum)),
            Err(e) if e.kind() == ErrorKind::NotFound => Ok(None),
            Err(e) => Err(Error::new(e).context(format!(
                "Failed reading checksum file {} for target {}",
                checksum_file.display(),
                identifier
            ))),
        }
    }

    fn erase_checksum(&self, identifier: &str) -> Result<()> {
        let checksum_file = &self.get_checksum_file(identifier);
        if checksum_file.exists() {
            fs::remove_file(&checksum_file).with_context(|| {
                format!(
                    "Failed to delete checksum file {} for target {}",
                    checksum_file.display(),
                    identifier
                )
            })?;
        }
        Ok(())
    }

    fn write_checksum(&self, identifier: &str, checksum: &Option<String>) -> Result<()> {
        if let Some(checksum) = checksum {
            let checksum_file = self.get_checksum_file(identifier);
            let mut file = fs::File::create(&checksum_file).with_context(|| {
                format!(
                    "Failed to create checksum file {} for target {}",
                    checksum_file.display(),
                    identifier
                )
            })?;
            file.write_all(checksum.as_bytes()).with_context(|| {
                format!(
                    "Failed to write checksum file {} for target {}",
                    checksum_file.display(),
                    identifier
                )
            })?;
        }

        Ok(())
    }
}

fn compute_checksum(identifier: &str, paths: &[PathBuf]) -> Result<Option<String>> {
    if paths.is_empty() {
        return Ok(None);
    }

    let computation_start = Instant::now();
    log::trace!("{} - Computing checksum", identifier);
    let mut hasher: XXHasher = Default::default();

    for path in paths.iter() {
        let checksum = calculate_path_checksum(path)?;
        checksum.unwrap_or(0).hash(&mut hasher);
    }

    let computation_duration = computation_start.elapsed();
    log::trace!(
        "{} - Checksum computed (took {}ms)",
        identifier,
        computation_duration.as_millis()
    );

    Ok(Some(hasher.finish().to_string()))
}

fn calculate_path_checksum(path: &Path) -> Result<Option<u64>> {
    if !path.exists() {
        return Ok(None);
    }

    let mut hasher: XXHasher = Default::default();

    for entry in WalkDir::new(path) {
        let entry = entry.with_context(|| "Failed to traverse directory")?;

        if entry.path().is_file() {
            let contents = fs::read(entry.path())
                .with_context(|| "Failed to read file to calculate checksum")?;
            contents.hash(&mut hasher);
        }
    }

    Ok(Some(hasher.finish()))
}
