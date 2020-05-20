use crate::config;
use crate::domain;
use anyhow::{Context, Result};
use std::collections::HashMap;
use std::path::PathBuf;

pub fn into_targets(
    mut parsed_targets: HashMap<String, (PathBuf, config::Target)>,
    requested_targets: Option<Vec<String>>,
) -> Result<Vec<domain::Target>> {
    let requested_targets =
        requested_targets.unwrap_or_else(|| parsed_targets.keys().cloned().collect());
    let mut targets = Vec::with_capacity(requested_targets.len());
    let mut mapping = HashMap::with_capacity(requested_targets.len());

    fn add_target(
        mut targets: &mut Vec<domain::Target>,
        mut mapping: &mut HashMap<String, domain::TargetId>,
        parsed_targets: &mut HashMap<String, (PathBuf, config::Target)>,
        target_name: &str,
    ) -> Result<()> {
        if mapping.contains_key(target_name) {
            return Ok(());
        }

        let (
            project_dir,
            config::Target {
                dependencies,
                input_paths,
                output_paths,
                build,
                service,
            },
        ) = parsed_targets
            .remove(target_name)
            .with_context(|| format!("Target {} does not exist", target_name))?;
        for dependency in &dependencies {
            add_target(&mut targets, &mut mapping, parsed_targets, dependency)?
        }

        let target_id = targets.len();
        mapping.insert(target_name.to_string(), target_id);
        let dependencies = dependencies
            .into_iter()
            .map(|target_name| *mapping.get(&target_name).unwrap())
            .collect();
        let input_paths = input_paths
            .into_iter()
            .map(|path| project_dir.join(path))
            .collect();
        let output_paths = output_paths
            .into_iter()
            .map(|path| project_dir.join(path))
            .collect();
        targets.push(domain::Target {
            id: target_id,
            name: target_name.to_string(),
            dependencies,
            path: project_dir.to_path_buf(),
            input_paths,
            output_paths,
            build,
            service,
        });

        Ok(())
    }

    for requested_target in requested_targets.iter() {
        add_target(
            &mut targets,
            &mut mapping,
            &mut parsed_targets,
            requested_target,
        )?;
    }

    Ok(targets)
}

#[cfg(test)]
mod tests {
    use super::into_targets;
    use crate::config::tests::build_targets;
    use crate::config::Target;

    #[test]
    fn test_into_targets_should_return_the_requested_targets() {
        let targets = build_targets(vec![
            ("target_1", build_target()),
            ("target_2", build_target()),
        ]);

        let actual_targets = into_targets(targets, Some(vec!["target_2".to_string()]))
            .expect("Conversion of valid targets should be successful");

        assert_eq!(actual_targets.len(), 1);
        assert_eq!(actual_targets[0].name, "target_2");
    }

    #[test]
    fn test_into_targets_should_reject_requested_target_not_found() {
        let targets = build_targets(vec![("target_1", build_target())]);

        into_targets(targets, Some(vec!["not_a_target".to_string()]))
            .expect_err("Should reject an invalid requested target");
    }

    fn build_target() -> Target {
        Target {
            dependencies: vec![],
            input_paths: vec![],
            output_paths: vec![],
            build: None,
            service: None,
        }
    }
}
