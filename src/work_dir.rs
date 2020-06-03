use anyhow::{Error, Result};
use std::io::ErrorKind;
use std::path::{self, Path, PathBuf};

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
    use std::path::Path;

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

pub fn remove_work_dir(project_dir: PathBuf) -> Result<()> {
    let checksums_dir = get_work_dir_path(&project_dir);
    match std::fs::remove_dir_all(&checksums_dir) {
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
