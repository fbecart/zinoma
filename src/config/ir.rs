use super::yaml;
use crate::domain::{self, TargetCanonicalName};
use anyhow::{anyhow, Result};
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
        root_target_names: Vec<TargetCanonicalName>,
    ) -> Result<(Vec<domain::Target>, Vec<domain::TargetId>)> {
        fn add_target(
            mut domain_targets: &mut Vec<domain::Target>,
            mut target_id_mapping: &mut HashMap<TargetCanonicalName, domain::TargetId>,
            config: &mut Config,
            target_canonical_name: TargetCanonicalName,
            parent_targets: &[&TargetCanonicalName],
        ) -> Result<domain::TargetId> {
            if let Some(&target_id) = target_id_mapping.get(&target_canonical_name) {
                return Ok(target_id);
            }

            if parent_targets.contains(&&target_canonical_name) {
                return Err(anyhow!(
                    "Circular dependency: {} -> {}",
                    itertools::join(parent_targets, " -> "),
                    target_canonical_name
                ));
            }

            let (project_dir, yaml_target) = {
                let (project_dir, project) = config
                    .projects
                    .get_mut(&target_canonical_name.project_name)
                    .ok_or_else(|| {
                        anyhow!(
                            "Project {} does not exist",
                            target_canonical_name.project_name.as_ref().unwrap()
                        )
                    })?;

                let yaml_target = project
                    .targets
                    .remove(&target_canonical_name.target_name)
                    .ok_or_else(|| anyhow!("Target {} does not exist", target_canonical_name))?;

                (project_dir.clone(), yaml_target)
            };

            let mut dependencies = TargetCanonicalName::try_parse_many(
                get_dependencies(&yaml_target),
                &target_canonical_name.project_name,
            )?;

            let (mut target_type, dependencies_from_input) =
                into_target_type(yaml_target, &target_canonical_name, &project_dir)?;

            dependencies.extend_from_slice(&dependencies_from_input);

            let targets_chain = [parent_targets, &[&target_canonical_name]].concat();
            let dependencies = dependencies
                .into_iter()
                .map(|dependency| {
                    add_target(
                        &mut domain_targets,
                        &mut target_id_mapping,
                        config,
                        dependency,
                        &targets_chain,
                    )
                })
                .collect::<Result<Vec<_>>>()?;

            for dependency_name in &dependencies_from_input {
                let dependency = &domain_targets[target_id_mapping[&dependency_name]];

                if let domain::TargetType::Build { output, .. } = &dependency.target_type {
                    target_type.extend_input(output).unwrap();
                } else {
                    return Err(anyhow!(
                        "Target {} can not depend on {}'s output as it is not a build target",
                        target_canonical_name,
                        dependency_name
                    ));
                };
            }

            let target_id = domain_targets.len();
            target_id_mapping.insert(target_canonical_name.clone(), target_id);

            domain_targets.push(domain::Target {
                id: target_id,
                name: target_canonical_name,
                project_dir,
                dependencies,
                target_type,
            });

            Ok(target_id)
        }

        let mut domain_targets = Vec::with_capacity(root_target_names.len());
        let mut root_target_ids = Vec::with_capacity(root_target_names.len());
        let mut target_id_mapping = HashMap::with_capacity(root_target_names.len());

        for target in root_target_names.into_iter() {
            root_target_ids.push(add_target(
                &mut domain_targets,
                &mut target_id_mapping,
                &mut self,
                target,
                &[],
            )?);
        }

        Ok((domain_targets, root_target_ids))
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

    fn get_project(&self, project_name: &Option<String>) -> &yaml::Project {
        &self.projects[&project_name].1
    }
}

pub fn get_dependencies(target: &yaml::Target) -> &Vec<String> {
    &match target {
        yaml::Target::Build { dependencies, .. } => dependencies,
        yaml::Target::Service { dependencies, .. } => dependencies,
        yaml::Target::Aggregate { dependencies } => dependencies,
    }
    .0
}

fn into_target_type(
    yaml_target: yaml::Target,
    target_canonical_name: &TargetCanonicalName,
    project_dir: &Path,
) -> Result<(domain::TargetType, Vec<TargetCanonicalName>)> {
    match yaml_target {
        yaml::Target::Build {
            build,
            input,
            output,
            ..
        } => {
            let (input, dependencies_from_input) =
                transform_input(input, target_canonical_name, project_dir)?;
            let output = transform_output(output, project_dir);
            Ok((
                domain::TargetType::Build {
                    build_script: build,
                    input,
                    output,
                },
                dependencies_from_input,
            ))
        }
        yaml::Target::Service { service, input, .. } => {
            let (input, dependencies_from_input) =
                transform_input(input, target_canonical_name, project_dir)?;
            Ok((
                domain::TargetType::Service {
                    run_script: service,
                    input,
                },
                dependencies_from_input,
            ))
        }
        yaml::Target::Aggregate { .. } => Ok((domain::TargetType::Aggregate, vec![])),
    }
}

fn transform_input(
    input: yaml::InputResources,
    target_canonical_name: &TargetCanonicalName,
    project_dir: &Path,
) -> Result<(domain::Resources, Vec<TargetCanonicalName>)> {
    input.0.into_iter().fold(
        Ok((domain::Resources::new(), Vec::new())),
        |acc, resource| {
            let (mut input, mut dependencies_from_input) = acc?;

            use yaml::InputResource::*;
            match resource {
                Paths { paths } => {
                    let paths = paths.iter().map(|path| project_dir.join(path));
                    input.paths.extend(paths)
                }
                CmdStdout { cmd_stdout } => input.cmds.push((cmd_stdout, project_dir.to_owned())),
                DependencyOutput(id) => {
                    lazy_static! {
                        static ref RE: Regex =
                            Regex::new(r"^((\w[-\w]*::)?\w[-\w]*)\.output$").unwrap();
                    }
                    if let Some(captures) = RE.captures(&id) {
                        let dependency_canonical_name = TargetCanonicalName::try_parse(
                            captures.get(1).unwrap().as_str(),
                            &target_canonical_name.project_name,
                        )
                        .unwrap();
                        dependencies_from_input.push(dependency_canonical_name);
                    } else {
                        return Err(anyhow!("Invalid input: {}", id));
                    }
                }
            }
            Ok((input, dependencies_from_input))
        },
    )
}

fn transform_output(output: yaml::OutputResources, project_dir: &Path) -> domain::Resources {
    output
        .0
        .into_iter()
        .fold(domain::Resources::new(), |mut acc, resource| {
            use yaml::OutputResource::*;
            match resource {
                Paths { paths } => {
                    let paths = paths.iter().map(|path| project_dir.join(path));
                    acc.paths.extend(paths)
                }
                CmdStdout { cmd_stdout } => acc.cmds.push((cmd_stdout, project_dir.to_owned())),
            }
            acc
        })
}

#[cfg(test)]
mod tests {
    use super::Config;
    use crate::config::yaml;
    use crate::domain::{self, TargetCanonicalName};
    use std::collections::HashMap;
    use std::path::PathBuf;

    #[test]
    fn test_try_into_domain_targets_should_return_the_requested_targets() {
        let projects = build_config(vec![
            ("target_1", build_empty_target()),
            ("target_2", build_empty_target()),
        ]);

        let (actual_targets, _) = projects
            .try_into_domain_targets(build_target_canonical_names(vec!["target_2"]))
            .expect("Conversion of valid targets should be successful");

        assert_eq!(actual_targets.len(), 1);
        assert_eq!(actual_targets[0].name.target_name, "target_2");
    }

    #[test]
    fn test_try_into_domain_targets_should_reject_requested_target_not_found() {
        let projects = build_config(vec![("target_1", build_empty_target())]);

        projects
            .try_into_domain_targets(build_target_canonical_names(vec!["not_a_target"]))
            .expect_err("Should reject an invalid requested target");
    }

    #[test]
    fn test_try_into_domain_targets_with_dependency_input() {
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

        let (actual_targets, _) = config
            .try_into_domain_targets(build_target_canonical_names(vec!["target_2"]))
            .unwrap();

        assert_eq!(actual_targets.len(), 2);
        let target1 = find_target(&actual_targets, "target_1");
        let target2 = find_target(&actual_targets, "target_2");
        assert_eq!(target2.dependencies, vec![target1.id]);
        assert_eq!(target2.get_input(), target1.get_output());
    }

    #[test]
    fn test_try_into_domain_targets_on_valid_targets() {
        let projects = build_config(vec![
            ("target_1", build_target_with_dependencies(vec!["target_2"])),
            ("target_2", build_empty_target()),
        ]);

        projects
            .try_into_domain_targets(build_target_canonical_names(vec!["target_1", "target_2"]))
            .expect("Valid targets should be accepted");
    }

    #[test]
    fn test_try_into_domain_targets_with_unknown_dependency() {
        let projects = build_config(vec![(
            "target_1",
            build_target_with_dependencies(vec!["target_2"]),
        )]);

        projects
            .try_into_domain_targets(build_target_canonical_names(vec!["target_1"]))
            .expect_err("Unknown dependencies should be rejected");
    }

    #[test]
    fn test_try_into_domain_targets_with_circular_dependency() {
        let projects = build_config(vec![
            ("target_1", build_target_with_dependencies(vec!["target_2"])),
            ("target_2", build_target_with_dependencies(vec!["target_3"])),
            ("target_3", build_target_with_dependencies(vec!["target_1"])),
        ]);

        projects
            .try_into_domain_targets(build_target_canonical_names(vec![
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

    fn build_empty_target() -> yaml::Target {
        build_target_with_dependencies(vec![])
    }

    fn build_target_with_dependencies(dependencies: Vec<&str>) -> yaml::Target {
        yaml::Target::Aggregate {
            dependencies: yaml::Dependencies(
                dependencies.into_iter().map(str::to_string).collect(),
            ),
        }
    }

    fn build_target_with_input(input: Vec<yaml::InputResource>) -> yaml::Target {
        yaml::Target::Build {
            dependencies: yaml::Dependencies(vec![]),
            build: ":".to_string(),
            input: yaml::InputResources(input),
            output: yaml::OutputResources(vec![]),
        }
    }

    fn build_target_with_output(output: Vec<yaml::OutputResource>) -> yaml::Target {
        yaml::Target::Build {
            dependencies: yaml::Dependencies(vec![]),
            build: ":".to_string(),
            input: yaml::InputResources(vec![]),
            output: yaml::OutputResources(output),
        }
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
