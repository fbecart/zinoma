use super::yaml;
use crate::domain;
use anyhow::Context;
use anyhow::Result;
use std::collections::HashMap;
use std::path::PathBuf;

pub struct Config {
    root_project_name: Option<String>,
    projects: HashMap<Option<String>, (PathBuf, yaml::Project)>,
}

impl From<yaml::Config> for Config {
    fn from(config: yaml::Config) -> Self {
        let yaml::Config {
            root_project_dir,
            projects,
        } = config;
        Self {
            root_project_name: projects[&root_project_dir].name.to_owned(),
            projects: projects
                .into_iter()
                .map(|(project_dir, project)| (project.name.clone(), (project_dir, project)))
                .collect(),
        }
    }
}

impl Config {
    pub fn list_all_target_names(&self) -> Vec<String> {
        let mut target_names: Vec<_> = self.projects[&self.root_project_name]
            .1
            .targets
            .keys()
            .cloned()
            .collect();
        target_names.extend(
            self.projects
                .iter()
                .filter(|(project_name, _)| project_name.is_some())
                .flat_map(|(project_name, (_project_dir, project))| {
                    project
                        .targets
                        .keys()
                        .map(|target_name| format_target_canonical_name(&project_name, target_name))
                        .collect::<Vec<_>>()
                }),
        );
        target_names
    }

    pub fn try_into_domain_targets(
        self,
        requested_targets: Option<Vec<String>>,
    ) -> Result<Vec<domain::Target>> {
        let requested_targets = requested_targets.unwrap_or_else(|| self.list_all_target_names());

        self.validate_dependency_graph(&requested_targets)?;

        let Config {
            root_project_name,
            mut projects,
        } = self;
        let mut targets = Vec::with_capacity(requested_targets.len());
        let mut mapping = HashMap::with_capacity(requested_targets.len());

        fn add_target(
            mut targets: &mut Vec<domain::Target>,
            mut mapping: &mut HashMap<(Option<String>, String), domain::TargetId>,
            projects: &mut HashMap<Option<String>, (PathBuf, yaml::Project)>,
            target_canonical_name: &(Option<String>, String),
        ) -> Result<()> {
            if mapping.contains_key(&target_canonical_name) {
                return Ok(());
            }

            let (project_name, target_name) = target_canonical_name;
            let project_dir = projects[&project_name].0.clone();
            let yaml::Target {
                dependencies,
                input_paths,
                output_paths,
                build,
                service,
            } = projects
                .get_mut(&project_name)
                .unwrap()
                .1
                .targets
                .remove(target_name)
                .unwrap();

            let dependencies = dependencies
                .into_iter()
                .map(|dependency_canonical_name| {
                    parse_target_canonical_name(&dependency_canonical_name).map(
                        |(dependency_project_name, dependency_name)| {
                            let dependency_project_name =
                                dependency_project_name.or_else(|| project_name.to_owned());
                            (dependency_project_name, dependency_name)
                        },
                    )
                })
                .collect::<Result<Vec<(_, _)>>>()?;

            for dependency in &dependencies {
                add_target(&mut targets, &mut mapping, projects, dependency)?
            }

            let target_id = targets.len();
            mapping.insert((project_name.clone(), target_name.clone()), target_id);
            let dependencies = dependencies
                .into_iter()
                .map(|(dependency_project_name, dependency_name)| {
                    *mapping
                        .get(&(dependency_project_name, dependency_name))
                        .unwrap()
                })
                .collect();
            let input_paths = input_paths
                .into_iter()
                .map(|path| (&project_dir).join(path))
                .collect();
            let output_paths = output_paths
                .into_iter()
                .map(|path| (&project_dir).join(path))
                .collect();
            targets.push(domain::Target {
                id: target_id,
                name: target_name.clone(),
                project: domain::Project {
                    dir: project_dir,
                    name: project_name.clone(),
                },
                dependencies,
                input_paths,
                output_paths,
                build,
                service,
            });

            Ok(())
        }

        for requested_target in requested_targets.iter() {
            let (pj_name, tg_name) = parse_target_canonical_name(requested_target)?;
            let pj_name = pj_name.or_else(|| root_project_name.clone());
            add_target(
                &mut targets,
                &mut mapping,
                &mut projects,
                &(pj_name, tg_name),
            )?;
        }

        Ok(targets)
    }

    /// Checks the validity of the provided targets.
    ///
    /// Ensures that all target dependencies (both direct and transitive) exist,
    /// and that the dependency graph has no circular dependency.
    fn validate_dependency_graph(&self, requested_targets: &[String]) -> Result<()> {
        for target_canonical_name in requested_targets {
            let (project_name, target_name) = parse_target_canonical_name(target_canonical_name)?;
            let project_name = project_name.or_else(|| self.root_project_name.clone());
            let target = self
                .projects
                .get(&project_name)
                .unwrap()
                .1
                .targets
                .get(&target_name)
                .ok_or_else(|| anyhow::anyhow!("Target {} not found", target_canonical_name))?;
            self.validate_target_graph(
                &(project_name, target_name.to_owned()),
                &target,
                &[],
                &self.root_project_name,
            )
            .with_context(|| format!("Target {} is invalid", target_name))?;
        }

        Ok(())
    }

    fn validate_target_graph(
        &self,
        target_canonical_name: &(Option<String>, String),
        target: &yaml::Target,
        parent_targets: &[&(Option<String>, String)],
        current_project: &Option<String>,
    ) -> Result<()> {
        if parent_targets.contains(&target_canonical_name) {
            return Err(anyhow::anyhow!(
                "Circular dependency: {} -> {}",
                parent_targets
                    .iter()
                    .map(|(project_name, target_name)| format_target_canonical_name(
                        &project_name,
                        &target_name
                    ))
                    .collect::<Vec<_>>()
                    .join(" -> "),
                format_target_canonical_name(&target_canonical_name.0, &target_canonical_name.1),
            ));
        }

        let targets_chain = [parent_targets, &[target_canonical_name]].concat();
        for dependency_canonical_name in &target.dependencies {
            let dependency_canonical_name = parse_target_canonical_name(dependency_canonical_name)?;
            let dependency_project = dependency_canonical_name
                .0
                .clone()
                .or_else(|| current_project.clone());
            let dependency = self
                .projects
                .get(&dependency_project)
                .unwrap()
                .1
                .targets
                .get(&dependency_canonical_name.1)
                .ok_or_else(|| {
                    anyhow::anyhow!(
                        "{} - Dependency {} not found",
                        format_target_canonical_name(
                            &target_canonical_name.0,
                            &target_canonical_name.1
                        ),
                        format_target_canonical_name(
                            &dependency_canonical_name.0,
                            &dependency_canonical_name.1
                        ),
                    )
                })?;

            self.validate_target_graph(
                &dependency_canonical_name,
                dependency,
                &targets_chain,
                &target_canonical_name.0,
            )?;
        }

        Ok(())
    }
}

fn format_target_canonical_name(project_name: &Option<String>, target_name: &str) -> String {
    if let Some(project_name) = project_name {
        format!("{}::{}", project_name, target_name)
    } else {
        target_name.to_owned()
    }
}

fn parse_target_canonical_name(target_canonical_name: &str) -> Result<(Option<String>, String)> {
    let parts = target_canonical_name.split("::").collect::<Vec<_>>();
    match parts[..] {
        [project_name, target_name] => Ok((Some(project_name.to_owned()), target_name.to_owned())),
        [target_name] => Ok((None, target_name.to_owned())),
        _ => Err(anyhow::anyhow!(
            "Invalid target canonical name: {} (expected a maximum of one '::' delimiter)",
            target_canonical_name
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::Config;
    use crate::config::yaml;
    use std::collections::HashMap;
    use std::path::PathBuf;

    #[test]
    fn test_into_targets_should_return_the_requested_targets() {
        let projects = build_projects(vec![
            ("target_1", build_target()),
            ("target_2", build_target()),
        ]);

        let actual_targets = projects
            .try_into_domain_targets(Some(vec!["target_2".to_string()]))
            .expect("Conversion of valid targets should be successful");

        assert_eq!(actual_targets.len(), 1);
        assert_eq!(actual_targets[0].name, "target_2");
    }

    #[test]
    fn test_into_targets_should_reject_requested_target_not_found() {
        let projects = build_projects(vec![("target_1", build_target())]);

        projects
            .try_into_domain_targets(Some(vec!["not_a_target".to_string()]))
            .expect_err("Should reject an invalid requested target");
    }

    #[test]
    fn test_validate_targets_on_valid_targets() {
        let projects = build_projects(vec![
            ("target_1", build_target_with_dependencies(vec!["target_2"])),
            ("target_2", build_target_with_dependencies(vec![])),
        ]);

        projects
            .validate_dependency_graph(&vec!["target_1".to_string(), "target_2".to_string()])
            .expect("Valid targets should be accepted");
    }

    #[test]
    fn test_validate_targets_with_unknown_dependency() {
        let projects = build_projects(vec![(
            "target_1",
            build_target_with_dependencies(vec!["target_2"]),
        )]);

        projects
            .validate_dependency_graph(&vec!["target_1".to_string()])
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
