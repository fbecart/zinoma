use super::Target;
use anyhow::{Context, Result};
use lazy_static::lazy_static;
use regex::Regex;
use std::collections::HashMap;

pub fn validate_targets(targets: &HashMap<String, Target>) -> Result<()> {
    for (target_name, target) in targets.iter() {
        if !is_valid_target_name(target_name) {
            return Err(anyhow::anyhow!(
                "{} is not a valid target name",
                target_name
            ));
        }

        validate_target(target_name, target, &[], targets)
            .with_context(|| format!("Target {} is invalid", target_name))?;
    }

    Ok(())
}

pub fn is_valid_target_name(target_name: &str) -> bool {
    lazy_static! {
        static ref RE: Regex = Regex::new(r"^\w[-\w]*$").unwrap();
    }
    RE.is_match(target_name)
}

/// Checks the validity of the provided target.
///
/// Ensures that all target dependencies (both direct and transitive) exist,
/// and that the dependency graph has no circular dependency.
fn validate_target(
    target_name: &str,
    target: &Target,
    parent_targets: &[&str],
    targets: &HashMap<String, Target>,
) -> Result<()> {
    if parent_targets.contains(&target_name) {
        return Err(anyhow::anyhow!(
            "Circular dependency: {} -> {}",
            parent_targets.join(" -> "),
            target_name
        ));
    }

    let targets_chain = [parent_targets, &[target_name]].concat();
    for dependency_name in &target.dependencies {
        let dependency = targets.get(dependency_name).ok_or_else(|| {
            anyhow::anyhow!("{} - Dependency {} not found", target_name, dependency_name)
        })?;

        validate_target(dependency_name, dependency, &targets_chain, &targets)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::is_valid_target_name;
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

    #[test]
    fn test_is_valid_target_name() {
        assert!(
            is_valid_target_name("my-target"),
            "A target name can contain letters and hyphens"
        );
        assert!(
            is_valid_target_name("007"),
            "A target name can contain numbers"
        );
        assert!(
            is_valid_target_name("_hidden_target"),
            "A target name can start with underscore"
        );

        assert!(
            !is_valid_target_name("-"),
            "A target name cannot start with an hyphen"
        );
        assert!(!is_valid_target_name(""), "A target name cannot be empty");
    }

    fn build_target_with_dependencies(dependencies: Vec<&str>) -> Target {
        Target {
            dependencies: dependencies.into_iter().map(str::to_string).collect(),
            input_paths: vec![],
            output_paths: vec![],
            build: None,
            service: None,
        }
    }
}
