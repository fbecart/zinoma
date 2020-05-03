mod conversion;
mod validation;

use crate::target;
use anyhow::{Context, Result};
use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use validation::{validate_requested_targets, validate_targets};

#[derive(Debug, Deserialize)]
pub struct Target {
    #[serde(default)]
    dependencies: Vec<String>,
    #[serde(default)]
    input_paths: Vec<String>,
    #[serde(default)]
    output_paths: Vec<String>,
    #[serde(default, rename = "build")]
    build_list: Vec<String>,
    #[serde(default)]
    service: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct Config {
    #[serde(default)]
    targets: HashMap<String, Target>,
}

impl Config {
    pub fn load(project_dir: &Path) -> Result<Self> {
        let config_file = project_dir.join("buildy.yml");
        let contents = fs::read_to_string(&config_file)
            .with_context(|| format!("Something went wrong reading {}", config_file.display()))?;
        let config: Self = serde_yaml::from_str(&contents)
            .with_context(|| format!("Invalid format for {}", config_file.display()))?;

        validate_targets(&config.targets).with_context(|| {
            format!(
                "Invalid configuration found in file {}",
                config_file.display()
            )
        })?;

        Ok(config)
    }

    pub fn into_targets(
        self,
        project_dir: &Path,
        requested_targets: &[String],
    ) -> Result<Vec<target::Target>> {
        validate_requested_targets(requested_targets, &self.targets)?;

        conversion::into_targets(self.targets, project_dir, requested_targets)
    }
}

#[cfg(test)]
mod tests {
    use super::Target;
    use std::collections::HashMap;

    pub fn build_targets(data: Vec<(&str, Target)>) -> HashMap<String, Target> {
        data.into_iter().map(|(k, v)| (k.to_string(), v)).collect()
    }
}
