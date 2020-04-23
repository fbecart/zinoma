use crypto::digest::Digest;
use crypto::sha1::Sha1;
use std::fs;
use std::io::Write;
use walkdir::WalkDir;

const CHECKSUM_DIRECTORY: &str = ".buildy";

pub enum IncrementalRunResult<T> {
    Skipped,
    Run(T),
}

pub fn run_incrementally<T, E, F>(
    name: &str,
    watch_list: &[String],
    my_fn: F,
) -> Result<IncrementalRunResult<Result<T, E>>, String>
where
    F: Fn() -> Result<T, E>,
{
    let watch_checksum = if watch_list.is_empty() {
        None
    } else {
        let mut hasher = Sha1::new();

        for path in watch_list.iter() {
            let checksum = calculate_checksum(path)?;
            hasher.input_str(&checksum);
        }

        Some(hasher.result_str())
    };

    if let Some(watch_checksum) = &watch_checksum {
        if does_checksum_match(name, &watch_checksum)? {
            return Ok(IncrementalRunResult::Skipped);
        }
    }

    reset_checksum(name)?;

    let result = my_fn();

    if result.is_ok() {
        if let Some(watch_checksum) = watch_checksum {
            write_checksum(name, &watch_checksum)?;
        }
    }

    Ok(IncrementalRunResult::Run(result))
}

fn calculate_checksum(path: &str) -> Result<String, String> {
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

fn checksum_file_name(target: &str) -> String {
    format!("{}/{}.checksum", CHECKSUM_DIRECTORY, target)
}

fn does_checksum_match(target: &str, checksum: &str) -> Result<bool, String> {
    // Might want to check for some errors like permission denied.
    fs::create_dir(CHECKSUM_DIRECTORY).ok();
    let file_name = checksum_file_name(target);
    match fs::read_to_string(&file_name) {
        Ok(old_checksum) => Ok(*checksum == old_checksum),
        Err(e) => {
            if e.kind() == std::io::ErrorKind::NotFound {
                // No checksum found.
                Ok(false)
            } else {
                Err(format!(
                    "Failed reading checksum file {} for target {}: {}",
                    file_name, target, e
                ))
            }
        }
    }
}

fn reset_checksum(target: &str) -> Result<(), String> {
    let file_name = checksum_file_name(target);
    if std::path::Path::new(&file_name).exists() {
        fs::remove_file(&file_name).map_err(|_| {
            format!(
                "Failed to delete checksum file {} for target {}",
                file_name, target
            )
        })?;
    }
    Ok(())
}

fn write_checksum(target: &str, checksum: &str) -> Result<(), String> {
    let file_name = checksum_file_name(target);
    let mut file = fs::File::create(&file_name).map_err(|_| {
        format!(
            "Failed to create checksum file {} for target {}",
            file_name, target
        )
    })?;
    file.write_all(checksum.as_bytes()).map_err(|_| {
        format!(
            "Failed to write checksum file {} for target {}",
            file_name, target
        )
    })?;
    Ok(())
}
