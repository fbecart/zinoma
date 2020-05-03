use super::Target;
use crate::config;
use anyhow::{Context, Result};
use std::collections::HashMap;

pub fn validate_targets(targets: &HashMap<String, Target>) -> Result<()> {
    for target_name in targets.keys() {
        validate_target(target_name, &[], targets)
            .with_context(|| format!("Target {} is invalid", target_name))?;
    }

    Ok(())
}

/// Checks the validity of the provided target.
///
/// Ensures that all target dependencies (both direct and transitive) exist,
/// and that the dependency graph has no circular dependency.
fn validate_target(
    target_name: &str,
    parent_targets: &[&str],
    targets: &HashMap<String, Target>,
) -> Result<()> {
    let target = targets
        .get(target_name)
        .ok_or_else(|| anyhow::anyhow!("Target {} not found", target_name))?;

    if parent_targets.contains(&target_name) {
        return Err(anyhow::anyhow!(
            "Circular dependency: {} -> {}",
            parent_targets.join(" -> "),
            target_name
        ));
    }

    let targets_chain = [parent_targets, &[target_name]].concat();
    for dependency in target.dependencies.iter() {
        validate_target(dependency, &targets_chain, &targets)?;
    }

    Ok(())
}

pub fn validate_requested_targets(
    requested_targets: &[String],
    targets: &HashMap<String, config::Target>,
) -> Result<()> {
    let invalid_targets: Vec<String> = requested_targets
        .iter()
        .filter(|&requested_target| !targets.contains_key(requested_target))
        .map(|i| i.to_owned())
        .collect();

    if !invalid_targets.is_empty() {
        return Err(anyhow::anyhow!(
            "Invalid targets: {}",
            invalid_targets.join(", ")
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::validate_targets;
    use crate::config::tests::build_targets;
    use crate::config::Target;

    #[test]
    fn test_validate_targets_on_valid_targets() {
        let targets = build_targets(vec![
            ("target_1", build_target_with_dependencies(vec!["target_2"])),
            ("target_2", build_target_with_dependencies(vec![])),
        ]);

        validate_targets(&targets).expect("Valid targets should be accepted");
    }

    #[test]
    fn test_validate_targets_with_unknown_dependency() {
        let targets = build_targets(vec![(
            "target_1",
            build_target_with_dependencies(vec!["target_2"]),
        )]);

        validate_targets(&targets).expect_err("Unknown dependencies should be rejected");
    }

    #[test]
    fn test_validate_targets_with_circular_dependency() {
        let targets = build_targets(vec![
            ("target_1", build_target_with_dependencies(vec!["target_2"])),
            ("target_2", build_target_with_dependencies(vec!["target_3"])),
            ("target_3", build_target_with_dependencies(vec!["target_1"])),
        ]);

        validate_targets(&targets).expect_err("Circular dependencies should be rejected");
    }

    fn build_target_with_dependencies(dependencies: Vec<&str>) -> Target {
        Target {
            dependencies: dependencies.iter().map(|&dep| dep.to_string()).collect(),
            input_paths: vec![],
            output_paths: vec![],
            build_list: vec![],
            service: None,
        }
    }
}
