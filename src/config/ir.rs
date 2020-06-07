use super::yaml;
use crate::domain::{self, TargetCanonicalName};
use anyhow::{anyhow, Context, Result};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

pub struct Config {
    pub root_project_name: Option<String>,
    projects: HashMap<Option<String>, (PathBuf, yaml::Project)>,
}

impl From<yaml::Config> for Config {
    fn from(config: yaml::Config) -> Self {
        Self {
            root_project_name: (&config.projects)[&config.root_project_dir].name.to_owned(),
            projects: config
                .projects
                .into_iter()
                .map(|(project_dir, project)| (project.name.clone(), (project_dir, project)))
                .collect(),
        }
    }
}

impl Config {
    pub fn list_all_available_target_names(&self) -> Vec<String> {
        let mut target_names = self
            .list_all_targets()
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>();

        if self.root_project_name.is_some() {
            // Add root project targets without their namespace
            target_names.extend(
                self.get_project(&self.root_project_name)
                    .targets
                    .keys()
                    .cloned(),
            );
        }

        target_names
    }

    pub fn try_into_domain_targets(
        mut self,
        root_targets: Vec<TargetCanonicalName>,
    ) -> Result<Vec<domain::Target>> {
        self.validate_dependency_graph(&root_targets)?;

        let mut domain_targets = Vec::with_capacity(root_targets.len());
        let mut target_id_mapping = HashMap::with_capacity(root_targets.len());

        fn add_target(
            mut domain_targets: &mut Vec<domain::Target>,
            mut target_id_mapping: &mut HashMap<TargetCanonicalName, domain::TargetId>,
            config: &mut Config,
            target_canonical_name: TargetCanonicalName,
        ) -> Result<domain::TargetId> {
            if let Some(&target_id) = target_id_mapping.get(&target_canonical_name) {
                return Ok(target_id);
            }

            let project_dir = config
                .get_project_dir(&target_canonical_name.project_name)
                .to_owned();
            let yaml_target = config
                .get_project_mut(&target_canonical_name.project_name)
                .targets
                .remove(&target_canonical_name.target_name)
                .unwrap();

            let dependency_ids = yaml_target
                .dependencies
                .iter()
                .map(|dependency| {
                    TargetCanonicalName::try_parse(&dependency, &target_canonical_name.project_name)
                        .and_then(|dependency| {
                            add_target(
                                &mut domain_targets,
                                &mut target_id_mapping,
                                config,
                                dependency,
                            )
                        })
                })
                .collect::<Result<Vec<_>>>()?;

            let target_id = domain_targets.len();
            target_id_mapping.insert(target_canonical_name.clone(), target_id);

            domain_targets.push(domain::Target::new(
                target_id,
                target_canonical_name,
                project_dir,
                dependency_ids,
                yaml_target,
            ));

            Ok(target_id)
        }

        for target in root_targets.into_iter() {
            add_target(
                &mut domain_targets,
                &mut target_id_mapping,
                &mut self,
                target,
            )?;
        }

        Ok(domain_targets)
    }

    pub fn list_all_targets(&self) -> Vec<TargetCanonicalName> {
        self.projects
            .iter()
            .flat_map(|(project_name, (_project_dir, project))| {
                project
                    .targets
                    .keys()
                    .map(|target_name| TargetCanonicalName {
                        project_name: project_name.to_owned(),
                        target_name: target_name.to_owned(),
                    })
                    .collect::<Vec<_>>()
            })
            .collect()
    }

    /// Checks the validity of the provided targets.
    ///
    /// Ensures that all target dependencies (both direct and transitive) exist,
    /// and that the dependency graph has no circular dependency.
    fn validate_dependency_graph(&self, root_targets: &[TargetCanonicalName]) -> Result<()> {
        for target_canonical_name in root_targets {
            let target = self
                .try_get_target(&target_canonical_name)
                .with_context(|| anyhow!("Target {} is invalid", &target_canonical_name))?;
            self.validate_target_graph(&target_canonical_name, &target, &[])
                .with_context(|| format!("Target {} is invalid", target_canonical_name))?;
        }

        Ok(())
    }

    fn validate_target_graph(
        &self,
        target_canonical_name: &TargetCanonicalName,
        target: &yaml::Target,
        parent_targets: &[&TargetCanonicalName],
    ) -> Result<()> {
        if parent_targets.contains(&target_canonical_name) {
            return Err(anyhow!(
                "Circular dependency: {} -> {}",
                parent_targets
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join(" -> "),
                &target_canonical_name,
            ));
        }

        let targets_chain = [parent_targets, &[target_canonical_name]].concat();
        for dependency_name in &target.dependencies {
            let dependency_canonical_name = TargetCanonicalName::try_parse(
                dependency_name,
                &target_canonical_name.project_name,
            )?;
            let dependency = self
                .try_get_target(&dependency_canonical_name)
                .with_context(|| {
                    anyhow!(
                        "{} - Dependency {} is invalid",
                        &target_canonical_name,
                        &dependency_canonical_name,
                    )
                })?;

            self.validate_target_graph(&dependency_canonical_name, dependency, &targets_chain)?;
        }

        Ok(())
    }

    fn get_project_dir<'a>(&'a self, project_name: &Option<String>) -> &'a Path {
        &self.projects[&project_name].0.as_ref()
    }

    fn get_project(&self, project_name: &Option<String>) -> &yaml::Project {
        &self.projects[&project_name].1
    }

    fn get_project_mut<'a>(&'a mut self, project_name: &Option<String>) -> &'a mut yaml::Project {
        &mut self.projects.get_mut(&project_name).unwrap().1
    }

    fn try_get_target(&self, target_canonical_name: &TargetCanonicalName) -> Result<&yaml::Target> {
        let project = match &self.projects.get(&target_canonical_name.project_name) {
            None => {
                return Err(anyhow!(
                    "Project {} does not exist",
                    target_canonical_name.project_name.to_owned().unwrap(),
                ))
            }
            Some((_project_dir, project)) => project,
        };

        match project.targets.get(&target_canonical_name.target_name) {
            None => Err(anyhow!("Target {} does not exist", target_canonical_name)),
            Some(target) => Ok(target),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Config;
    use crate::config::yaml;
    use crate::domain::TargetCanonicalName;
    use std::collections::HashMap;
    use std::path::PathBuf;

    #[test]
    fn test_into_targets_should_return_the_requested_targets() {
        let projects = build_projects(vec![
            ("target_1", build_target()),
            ("target_2", build_target()),
        ]);

        let actual_targets = projects
            .try_into_domain_targets(build_target_canonical_names(vec!["target_2"]))
            .expect("Conversion of valid targets should be successful");

        assert_eq!(actual_targets.len(), 1);
        assert_eq!(actual_targets[0].name.target_name, "target_2");
    }

    #[test]
    fn test_into_targets_should_reject_requested_target_not_found() {
        let projects = build_projects(vec![("target_1", build_target())]);

        projects
            .try_into_domain_targets(build_target_canonical_names(vec!["not_a_target"]))
            .expect_err("Should reject an invalid requested target");
    }

    #[test]
    fn test_validate_targets_on_valid_targets() {
        let projects = build_projects(vec![
            ("target_1", build_target_with_dependencies(vec!["target_2"])),
            ("target_2", build_target_with_dependencies(vec![])),
        ]);

        projects
            .validate_dependency_graph(&build_target_canonical_names(vec!["target_1", "target_2"]))
            .expect("Valid targets should be accepted");
    }

    #[test]
    fn test_validate_targets_with_unknown_dependency() {
        let projects = build_projects(vec![(
            "target_1",
            build_target_with_dependencies(vec!["target_2"]),
        )]);

        projects
            .validate_dependency_graph(&build_target_canonical_names(vec!["target_1"]))
            .expect_err("Unknown dependencies should be rejected");
    }

    #[test]
    fn test_validate_targets_with_circular_dependency() {
        let projects = build_projects(vec![
            ("target_1", build_target_with_dependencies(vec!["target_2"])),
            ("target_2", build_target_with_dependencies(vec!["target_3"])),
            ("target_3", build_target_with_dependencies(vec!["target_1"])),
        ]);

        projects
            .validate_dependency_graph(&build_target_canonical_names(vec![
                "target_1", "target_2", "target_3",
            ]))
            .expect_err("Circular dependencies should be rejected");
    }

    fn build_target_canonical_names(names: Vec<&str>) -> Vec<TargetCanonicalName> {
        names
            .iter()
            .map(|&target_name| TargetCanonicalName {
                project_name: None,
                target_name: target_name.to_owned(),
            })
            .collect()
    }

    fn build_target_with_dependencies(dependencies: Vec<&str>) -> yaml::Target {
        yaml::Target {
            dependencies: dependencies.into_iter().map(str::to_string).collect(),
            input: vec![],
            output: vec![],
            build: None,
            service: None,
        }
    }

    fn build_target() -> yaml::Target {
        build_target_with_dependencies(vec![])
    }

    pub fn build_projects(targets: Vec<(&str, yaml::Target)>) -> Config {
        let mut projects = HashMap::new();
        projects.insert(
            None,
            (
                PathBuf::new(),
                yaml::Project {
                    name: None,
                    imports: HashMap::new(),
                    targets: targets
                        .into_iter()
                        .map(|(k, v)| (k.to_string(), v))
                        .collect(),
                },
            ),
        );

        Config {
            root_project_name: None,
            projects,
        }
    }
}
