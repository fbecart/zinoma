use anyhow::{Error, Result};
use async_std::fs;
use async_std::path::{self, Path, PathBuf};
use std::io::ErrorKind;

/// Name of the directory in which Žinoma stores its own files.
const WORK_DIR_NAME: &str = ".zinoma";

pub fn is_in_work_dir(path: &Path) -> bool {
    path.components().any(|component| match component {
        path::Component::Normal(name) => name == WORK_DIR_NAME,
        _ => false,
    })
}

#[cfg(test)]
mod tests {
    use super::is_in_work_dir;
    use async_std::path::Path;

    #[test]
    fn test_is_in_work_dir() {
        assert!(is_in_work_dir(Path::new(".zinoma/my/file.json")));
        assert!(is_in_work_dir(Path::new(
            "/my/project/.zinoma/my/file.json"
        )));
        assert!(!is_in_work_dir(Path::new("/my/file.json")));
    }
}

pub fn get_work_dir_path(project_dir: &Path) -> PathBuf {
    project_dir.join(WORK_DIR_NAME)
}

pub async fn remove_work_dir(project_dir: &Path) -> Result<()> {
    let checksums_dir = get_work_dir_path(project_dir);
    match fs::remove_dir_all(&checksums_dir).await {
        Ok(_) => {}
        Err(e) if e.kind() == ErrorKind::NotFound => {}
        Err(e) => {
            return Err(Error::new(e).context(format!(
                "Failed to remove checksums directory {}",
                checksums_dir.display()
            )));
        }
    }

    Ok(())
}

pub fn is_work_dir(entry: &walkdir::DirEntry) -> bool {
    entry
        .file_name()
        .to_str()
        .map(|file_name| file_name == WORK_DIR_NAME)
        .unwrap_or(false)
}
