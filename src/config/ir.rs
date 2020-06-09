use super::yaml;
use crate::domain::{self, TargetCanonicalName};
use anyhow::{anyhow, Context, Result};
use lazy_static::lazy_static;
use regex::Regex;
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
        // TODO Validate dependencies found in inputs!
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

            let (mut input, dependencies_from_input) = yaml_target.input.iter().fold(
                Ok((domain::Resources::new(), Vec::new())),
                |acc: Result<(domain::Resources, Vec<String>)>, resource| {
                    let (mut input, mut dependencies_from_input) = acc?;

                    use yaml::InputResource::*;
                    match resource {
                        Paths { paths } => input
                            .paths
                            .extend(paths.iter().map(|path| project_dir.join(path))),
                        CmdStdout { cmd_stdout } => input.cmds.push(cmd_stdout.to_string()),
                        DependencyOutput(id) => {
                            lazy_static! {
                                static ref RE: Regex =
                                    Regex::new(r"^((\w[-\w]*::)?\w[-\w]*)\.output$").unwrap();
                            }
                            if let Some(captures) = RE.captures(id) {
                                dependencies_from_input.push(captures[1].to_string());
                            } else {
                                return Err(anyhow!("Invalid input: {}", id));
                            }
                        }
                    }
                    Ok((input, dependencies_from_input))
                },
            )?;

            let output =
                yaml_target
                    .output
                    .iter()
                    .fold(domain::Resources::new(), |mut acc, resource| {
                        use yaml::OutputResource::*;
                        match resource {
                            Paths { paths } => acc
                                .paths
                                .extend(paths.iter().map(|path| project_dir.join(path))),
                            CmdStdout { cmd_stdout } => acc.cmds.push(cmd_stdout.to_string()),
                        }
                        acc
                    });

            let mut dependencies = TargetCanonicalName::try_parse_many(
                &yaml_target.dependencies,
                &target_canonical_name.project_name,
            )?;

            let dependencies_from_input = TargetCanonicalName::try_parse_many(
                &dependencies_from_input,
                &target_canonical_name.project_name,
            )?;

            dependencies.extend_from_slice(&dependencies_from_input);

            let dependency_ids = dependencies
                .into_iter()
                .map(|dependency| {
                    add_target(
                        &mut domain_targets,
                        &mut target_id_mapping,
                        config,
                        dependency,
                    )
                })
                .collect::<Result<Vec<_>>>()?;

            for dependency in dependencies_from_input {
                input.extend(&domain_targets[target_id_mapping[&dependency]].output);
            }

            let target_id = domain_targets.len();
            target_id_mapping.insert(target_canonical_name.clone(), target_id);

            domain_targets.push(domain::Target::new(
                target_id,
                target_canonical_name,
                project_dir,
                dependency_ids,
                input,
                output,
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
    use crate::domain::{self, TargetCanonicalName};
    use std::collections::HashMap;
    use std::path::PathBuf;

    #[test]
    fn test_into_targets_should_return_the_requested_targets() {
        let projects = build_config(vec![
            ("target_1", yaml::Target::new()),
            ("target_2", yaml::Target::new()),
        ]);

        let actual_targets = projects
            .try_into_domain_targets(build_target_canonical_names(vec!["target_2"]))
            .expect("Conversion of valid targets should be successful");

        assert_eq!(actual_targets.len(), 1);
        assert_eq!(actual_targets[0].name.target_name, "target_2");
    }

    #[test]
    fn test_into_targets_should_reject_requested_target_not_found() {
        let projects = build_config(vec![("target_1", yaml::Target::new())]);

        projects
            .try_into_domain_targets(build_target_canonical_names(vec!["not_a_target"]))
            .expect_err("Should reject an invalid requested target");
    }

    #[test]
    fn test_into_targets_with_dependency_input() {
        let config = build_config(vec![
            (
                "target_1",
                build_target_with_output(vec![yaml::OutputResource::Paths {
                    paths: vec!["output.txt".to_string()],
                }]),
            ),
            (
                "target_2",
                build_target_with_input(vec![yaml::InputResource::DependencyOutput(
                    "target_1.output".to_string(),
                )]),
            ),
        ]);

        let actual_targets = config
            .try_into_domain_targets(build_target_canonical_names(vec!["target_2"]))
            .unwrap();

        assert_eq!(actual_targets.len(), 2);
        let target1 = find_target(&actual_targets, "target_1");
        let target2 = find_target(&actual_targets, "target_2");
        assert_eq!(target2.dependencies, vec![target1.id]);
        assert_eq!(target2.input, target1.output);
    }

    #[test]
    fn test_validate_targets_on_valid_targets() {
        let projects = build_config(vec![
            ("target_1", build_target_with_dependencies(vec!["target_2"])),
            ("target_2", build_target_with_dependencies(vec![])),
        ]);

        projects
            .validate_dependency_graph(&build_target_canonical_names(vec!["target_1", "target_2"]))
            .expect("Valid targets should be accepted");
    }

    #[test]
    fn test_validate_targets_with_unknown_dependency() {
        let projects = build_config(vec![(
            "target_1",
            build_target_with_dependencies(vec!["target_2"]),
        )]);

        projects
            .validate_dependency_graph(&build_target_canonical_names(vec!["target_1"]))
            .expect_err("Unknown dependencies should be rejected");
    }

    #[test]
    fn test_validate_targets_with_circular_dependency() {
        let projects = build_config(vec![
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
        let mut target = yaml::Target::new();
        target.dependencies = dependencies.into_iter().map(str::to_string).collect();
        target
    }

    fn build_target_with_input(input: Vec<yaml::InputResource>) -> yaml::Target {
        let mut target = yaml::Target::new();
        target.input = input;
        target
    }

    fn build_target_with_output(output: Vec<yaml::OutputResource>) -> yaml::Target {
        let mut target = yaml::Target::new();
        target.output = output;
        target
    }

    pub fn build_config(targets: Vec<(&str, yaml::Target)>) -> Config {
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

    fn find_target<'a>(targets: &'a [domain::Target], target_name: &str) -> &'a domain::Target {
        targets
            .iter()
            .find(|&t| &t.name.target_name == target_name)
            .unwrap()
    }
}
