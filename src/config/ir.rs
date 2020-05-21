use super::yaml;
use super::yaml::Projects;
use crate::domain;
use anyhow::Context;
use anyhow::Result;
use std::collections::HashMap;
use std::convert::TryFrom;
use std::path::PathBuf;

pub struct Targets(HashMap<String, (PathBuf, yaml::Target)>);

impl TryFrom<Projects> for Targets {
    type Error = anyhow::Error;
    fn try_from(projects: Projects) -> Result<Self> {
        let mut targets: HashMap<String, (PathBuf, yaml::Target)> = HashMap::new();

        for (project_dir, project) in projects.0.into_iter() {
            for (target_name, target) in project.targets.into_iter() {
                if let Some((homonym_dir, _)) = targets.get(&target_name) {
                    return Err(anyhow::anyhow!(
                        "Projects {} and {} contain targets with the same name: {}. Please disambiguate.",
                        project_dir.display(),
                        homonym_dir.display(),
                        target_name,
                    ));
                }

                targets.insert(target_name, (project_dir.clone(), target));
            }
        }

        Ok(Self(targets))
    }
}

impl Targets {
    pub fn get_target_names(&self) -> Vec<String> {
        self.0.keys().cloned().collect()
    }

    pub fn try_into_domain_targets(
        self,
        requested_targets: Option<Vec<String>>,
    ) -> Result<Vec<domain::Target>> {
        let requested_targets =
            requested_targets.unwrap_or_else(|| self.0.keys().cloned().collect());

        self.validate_dependency_graph(&requested_targets)?;

        let mut yaml_targets = self.0;
        let mut targets = Vec::with_capacity(requested_targets.len());
        let mut mapping = HashMap::with_capacity(requested_targets.len());

        fn add_target(
            mut targets: &mut Vec<domain::Target>,
            mut mapping: &mut HashMap<String, domain::TargetId>,
            yaml_targets: &mut HashMap<String, (PathBuf, yaml::Target)>,
            target_name: &str,
        ) -> Result<()> {
            if mapping.contains_key(target_name) {
                return Ok(());
            }

            let (
                project_dir,
                yaml::Target {
                    dependencies,
                    input_paths,
                    output_paths,
                    build,
                    service,
                },
            ) = yaml_targets.remove(target_name).unwrap();
            for dependency in &dependencies {
                add_target(&mut targets, &mut mapping, yaml_targets, dependency)?
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
                path: project_dir,
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
                &mut yaml_targets,
                requested_target,
            )?;
        }

        Ok(targets)
    }

    /// Checks the validity of the provided targets.
    ///
    /// Ensures that all target dependencies (both direct and transitive) exist,
    /// and that the dependency graph has no circular dependency.
    fn validate_dependency_graph(&self, target_names: &[String]) -> Result<()> {
        for target_name in target_names {
            let (_, target) = self
                .0
                .get(target_name)
                .ok_or_else(|| anyhow::anyhow!("Target {} not found", target_name))?;
            self.validate_target_graph(target_name, &target, &[])
                .with_context(|| format!("Target {} is invalid", target_name))?;
        }

        Ok(())
    }

    fn validate_target_graph(
        &self,
        target_name: &str,
        target: &yaml::Target,
        parent_targets: &[&str],
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
            let (_, dependency) = self.0.get(dependency_name).ok_or_else(|| {
                anyhow::anyhow!("{} - Dependency {} not found", target_name, dependency_name)
            })?;

            self.validate_target_graph(dependency_name, dependency, &targets_chain)?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::Targets;
    use crate::config::yaml;
    use std::path::PathBuf;

    #[test]
    fn test_into_targets_should_return_the_requested_targets() {
        let targets = build_targets(vec![
            ("target_1", build_target()),
            ("target_2", build_target()),
        ]);

        let actual_targets = targets
            .try_into_domain_targets(Some(vec!["target_2".to_string()]))
            .expect("Conversion of valid targets should be successful");

        assert_eq!(actual_targets.len(), 1);
        assert_eq!(actual_targets[0].name, "target_2");
    }

    #[test]
    fn test_into_targets_should_reject_requested_target_not_found() {
        let targets = build_targets(vec![("target_1", build_target())]);

        targets
            .try_into_domain_targets(Some(vec!["not_a_target".to_string()]))
            .expect_err("Should reject an invalid requested target");
    }

    #[test]
    fn test_validate_targets_on_valid_targets() {
        let targets = build_targets(vec![
            ("target_1", build_target_with_dependencies(vec!["target_2"])),
            ("target_2", build_target_with_dependencies(vec![])),
        ]);

        targets
            .validate_dependency_graph(&vec!["target_1".to_string(), "target_2".to_string()])
            .expect("Valid targets should be accepted");
    }

    #[test]
    fn test_validate_targets_with_unknown_dependency() {
        let targets = build_targets(vec![(
            "target_1",
            build_target_with_dependencies(vec!["target_2"]),
        )]);

        targets
            .validate_dependency_graph(&vec!["target_1".to_string()])
            .expect_err("Unknown dependencies should be rejected");
    }

    #[test]
    fn test_validate_targets_with_circular_dependency() {
        let targets = build_targets(vec![
            ("target_1", build_target_with_dependencies(vec!["target_2"])),
            ("target_2", build_target_with_dependencies(vec!["target_3"])),
            ("target_3", build_target_with_dependencies(vec!["target_1"])),
        ]);

        targets
            .validate_dependency_graph(&vec![
                "target_1".to_string(),
                "target_2".to_string(),
                "target_3".to_string(),
            ])
            .expect_err("Circular dependencies should be rejected");
    }

    fn build_target_with_dependencies(dependencies: Vec<&str>) -> yaml::Target {
        yaml::Target {
            dependencies: dependencies.into_iter().map(str::to_string).collect(),
            input_paths: vec![],
            output_paths: vec![],
            build: None,
            service: None,
        }
    }

    fn build_target() -> yaml::Target {
        build_target_with_dependencies(vec![])
    }

    pub fn build_targets(data: Vec<(&str, yaml::Target)>) -> Targets {
        Targets(
            data.into_iter()
                .map(|(k, v)| (k.to_string(), (PathBuf::from("."), v)))
                .collect(),
        )
    }
}
