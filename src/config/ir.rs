use super::yaml;
use crate::domain::{self, TargetId};
use anyhow::{anyhow, Result};
use async_std::path::{Path, PathBuf};
use domain::{CmdResource, FilesResource};
use lazy_static::lazy_static;
use regex::Regex;
use std::collections::{BTreeSet, HashMap};

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
                .map(|(project_dir, project)| (project.name.clone(), (project_dir.into(), project)))
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
        root_target_ids: &[TargetId],
    ) -> Result<HashMap<domain::TargetId, domain::Target>> {
        fn add_target(
            mut domain_targets: &mut HashMap<domain::TargetId, domain::Target>,
            config: &mut Config,
            target_id: &TargetId,
            parent_targets: &[&TargetId],
        ) -> Result<()> {
            if domain_targets.contains_key(target_id) {
                return Ok(());
            }

            if parent_targets.contains(&target_id) {
                return Err(anyhow!(
                    "Circular dependency: {} -> {}",
                    itertools::join(parent_targets, " -> "),
                    target_id
                ));
            }

            let (project_dir, yaml_target) = {
                let (project_dir, project) = config
                    .projects
                    .get_mut(&target_id.project_name)
                    .ok_or_else(|| {
                        anyhow!(
                            "Project {} does not exist",
                            target_id.project_name.as_ref().unwrap()
                        )
                    })?;

                let yaml_target = project
                    .targets
                    .remove(&target_id.target_name)
                    .ok_or_else(|| anyhow!("Target {} does not exist", target_id))?;

                (project_dir.clone(), yaml_target)
            };

            let (mut target, dependencies_from_input) =
                transform_target(target_id, yaml_target, project_dir)?;

            target.extend_dependencies(&dependencies_from_input);

            let targets_chain = [parent_targets, &[target_id]].concat();
            for dependency_id in target.dependencies() {
                add_target(&mut domain_targets, config, dependency_id, &targets_chain)?
            }

            for dependency_id in &dependencies_from_input {
                let dependency = &domain_targets[dependency_id];

                if let domain::Target::Build(dependency) = dependency {
                    target.extend_input(&dependency.output).unwrap();
                } else {
                    return Err(anyhow!(
                        "Target {} can not depend on {}'s output as it is not a build target",
                        target_id,
                        dependency_id
                    ));
                };
            }

            domain_targets.insert(target_id.clone(), target);

            Ok(())
        }

        let mut domain_targets = HashMap::with_capacity(root_target_ids.len());

        for target_id in root_target_ids {
            add_target(&mut domain_targets, &mut self, target_id, &[])?
        }

        Ok(domain_targets)
    }

    pub fn list_all_targets(&self) -> Vec<TargetId> {
        self.projects
            .iter()
            .flat_map(|(project_name, (_project_dir, project))| {
                project
                    .targets
                    .keys()
                    .map(|target_name| TargetId {
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

fn transform_target(
    target_id: &TargetId,
    yaml_target: yaml::Target,
    project_dir: PathBuf,
) -> Result<(domain::Target, Vec<TargetId>)> {
    let dependencies =
        TargetId::try_parse_many(get_dependencies(&yaml_target), &target_id.project_name)?;

    let metadata = domain::TargetMetadata {
        id: target_id.clone(),
        project_dir,
        dependencies,
    };

    match yaml_target {
        yaml::Target::Build {
            build,
            input,
            output,
            ..
        } => {
            let (input, dependencies_from_input) =
                transform_input(input, &metadata.id, &metadata.project_dir)?;
            let output = transform_output(output, &metadata.project_dir);
            Ok((
                domain::Target::Build(domain::BuildTarget {
                    metadata,
                    build_script: build,
                    input,
                    output,
                }),
                dependencies_from_input,
            ))
        }
        yaml::Target::Service { service, input, .. } => {
            let (input, dependencies_from_input) =
                transform_input(input, &metadata.id, &metadata.project_dir)?;
            Ok((
                domain::Target::Service(domain::ServiceTarget {
                    metadata,
                    run_script: service,
                    input,
                }),
                dependencies_from_input,
            ))
        }
        yaml::Target::Aggregate { .. } => Ok((
            domain::Target::Aggregate(domain::AggregateTarget { metadata }),
            vec![],
        )),
    }
}

fn transform_input(
    input: yaml::InputResources,
    target_id: &TargetId,
    project_dir: &Path,
) -> Result<(domain::Resources, Vec<TargetId>)> {
    input.0.into_iter().fold(
        Ok((domain::Resources::new(), Vec::new())),
        |acc, resource| {
            let (mut input, mut dependencies_from_input) = acc?;

            use yaml::InputResource::*;
            match resource {
                Files { paths, extensions } => {
                    let paths = paths.iter().map(|path| project_dir.join(path)).collect();
                    let extensions = extensions
                        .map(|extensions| {
                            extensions
                                .into_iter()
                                .filter(|ext| !ext.is_empty())
                                .map(|ext| {
                                    if ext.starts_with(".") {
                                        ext
                                    } else {
                                        format!(".{}", ext)
                                    }
                                })
                                .collect::<BTreeSet<_>>()
                        })
                        .filter(|extensions| !extensions.is_empty());
                    input.files.push(FilesResource { paths, extensions })
                }
                CmdStdout { cmd_stdout } => input.cmds.push(CmdResource {
                    cmd: cmd_stdout,
                    dir: project_dir.to_owned(),
                }),
                DependencyOutput(id) => {
                    lazy_static! {
                        static ref RE: Regex =
                            Regex::new(r"^((\w[-\w]*::)?\w[-\w]*)\.output$").unwrap();
                    }
                    if let Some(captures) = RE.captures(&id) {
                        let dependency_id = TargetId::try_parse(
                            captures.get(1).unwrap().as_str(),
                            &target_id.project_name,
                        )
                        .unwrap();
                        dependencies_from_input.push(dependency_id);
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
                Files { paths } => acc.files.push(FilesResource {
                    paths: paths.iter().map(|path| project_dir.join(path)).collect(),
                    extensions: None,
                }),
                CmdStdout { cmd_stdout } => acc.cmds.push(CmdResource {
                    cmd: cmd_stdout,
                    dir: project_dir.to_owned(),
                }),
            }
            acc
        })
}

#[cfg(test)]
mod tests {
    use super::Config;
    use crate::config::yaml;
    use crate::domain::{self, TargetId};
    use async_std::path::PathBuf;
    use std::collections::HashMap;

    #[test]
    fn test_try_into_domain_targets_should_return_the_requested_targets() {
        let projects = build_config(vec![
            ("target_1", build_empty_target()),
            ("target_2", build_empty_target()),
        ]);

        let actual_targets = projects
            .try_into_domain_targets(&build_target_ids(vec!["target_2"]))
            .expect("Conversion of valid targets should be successful");

        assert_eq!(actual_targets.len(), 1);
        assert!(find_target(&actual_targets, "target_2").is_some());
    }

    #[test]
    fn test_try_into_domain_targets_should_reject_requested_target_not_found() {
        let projects = build_config(vec![("target_1", build_empty_target())]);

        projects
            .try_into_domain_targets(&build_target_ids(vec!["not_a_target"]))
            .expect_err("Should reject an invalid requested target");
    }

    #[test]
    fn test_try_into_domain_targets_with_dependency_input() {
        let config = build_config(vec![
            (
                "target_1",
                build_target_with_output(vec![yaml::OutputResource::Files {
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
            .try_into_domain_targets(&build_target_ids(vec!["target_2"]))
            .unwrap();

        assert_eq!(actual_targets.len(), 2);
        let target1 = find_target(&actual_targets, "target_1").unwrap();
        let target2 = find_target(&actual_targets, "target_2").unwrap();
        assert_eq!(target2.dependencies(), &vec![target1.id().clone()]);
        assert_eq!(target2.input(), target1.output());
    }

    #[test]
    fn test_try_into_domain_targets_on_valid_targets() {
        let projects = build_config(vec![
            ("target_1", build_target_with_dependencies(vec!["target_2"])),
            ("target_2", build_empty_target()),
        ]);

        projects
            .try_into_domain_targets(&build_target_ids(vec!["target_1", "target_2"]))
            .expect("Valid targets should be accepted");
    }

    #[test]
    fn test_try_into_domain_targets_with_unknown_dependency() {
        let projects = build_config(vec![(
            "target_1",
            build_target_with_dependencies(vec!["target_2"]),
        )]);

        projects
            .try_into_domain_targets(&build_target_ids(vec!["target_1"]))
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
            .try_into_domain_targets(&build_target_ids(vec!["target_1", "target_2", "target_3"]))
            .expect_err("Circular dependencies should be rejected");
    }

    fn build_target_ids(names: Vec<&str>) -> Vec<TargetId> {
        names
            .iter()
            .map(|&target_name| TargetId {
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

    fn find_target<'a>(
        targets: &'a HashMap<domain::TargetId, domain::Target>,
        target_name: &str,
    ) -> Option<&'a domain::Target> {
        targets.get(&TargetId {
            project_name: None,
            target_name: target_name.to_string(),
        })
    }
}
