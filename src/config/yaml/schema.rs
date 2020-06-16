use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// In order to use Žinoma with your project, you need to create a file named `zinoma.yml`.
/// We recommend putting this file in the root directory of your project.
///
/// This struct describes the schema expected for this file. It assumes prior knowledge of the Yaml format.
///
/// __Example__
///
/// `zinoma.yml`:
///
/// ```yaml
/// targets:
///   download_dependencies:
///     input:
///       - paths: [package.json, package-lock.json]
///     output:
///       - paths: [node_modules]
///     build: npm install
///
///   test:
///     input:
///       - download_dependencies.output
///       - paths: [package.json, src, test]
///     build: npm test
///
///   lint:
///     input:
///       - download_dependencies.output
///       - paths: [package.json, src, test]
///     build: npm run lint
///
///   check:
///     dependencies: [test, lint]
///
///   start:
///     input:
///       - download_dependencies.output
///       - paths: [package.json, src]
///     service: exec npm run start
///
///   build:
///     dependencies: [check]
///     input:
///       - paths:
///         - Dockerfile
///         - package.json
///         - package-lock.json
///         - src
///     output:
///       - paths: [lambda.zip]
///     build: |
///       docker build -t build-my-project:latest .
///       docker create -ti --name build-my-project build-my-project:latest bash
///       docker cp build-my-project:/var/task/lambda.zip ./
///       docker rm -f build-my-project
/// ```
///
/// In this example:
///
/// - `zinoma check` will ensure the code complies to the test suites and the coding standards.
/// - `zinoma start --watch` will run the application and restart it whenever the sources are updated.
/// - `zinoma --clean build` will generate a clean artifact, ready to be deployed.
///
/// A fully functional and more advanced example project is available in [fbecart/zinoma-node-example](https://github.com/fbecart/zinoma-node-example).
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Project {
    /// Targets (aka tasks) of this project.
    ///
    /// Each [`target`] is a unit of work to perform as part of the build flow.
    ///
    /// [`target`]: struct.Target.html
    ///
    /// Targets run in parallel by default.
    /// To run targets sequentially, you can define dependencies on other targets using the [`dependencies`] keyword.
    ///
    /// [`dependencies`]: struct.Target.html#structfield.dependencies
    ///
    /// Each target must have a unique name.
    /// The target name must be a string. It should start with an alphanumeric character or `_` and contain only alphanumeric characters, `-`, or `_`.
    ///
    /// __Example__
    ///
    /// ```yaml
    /// targets:
    ///   speak_cow:
    ///     build: echo 'Moo'
    ///   speak_dog:
    ///     build: echo 'Woof!'
    /// ```
    ///
    /// In this example:
    ///
    /// - `zinoma speak_cow` will print `Moo`
    /// - `zinoma speak_dog` will print `Woof!`
    /// - `zinoma speak_cow speak_dog` will print both `Moo` and `Woof!`, not necessarily in order.
    #[serde(default)]
    pub targets: HashMap<String, Target>,

    /// Name of the project.
    ///
    /// A project name must be a string. It should start with an alphanumeric character or `_` and contain only alphanumeric characters, `-`, or `_`.
    ///
    /// Project names should be unique. Two projects cannot have the same name.
    ///
    /// __Example__
    ///
    /// ```yaml
    /// name: my_project
    /// ```
    #[serde(default)]
    pub name: Option<String>,

    /// Import definitions from other Žinoma projects.
    ///
    /// `imports` should be an object, the keys being the project names and the values their respective paths.
    ///
    /// Before importing a project, you should make sure this project has its name defined.
    /// You should use the same name as key in the `imports` object.
    ///
    /// Once a project is imported, targets from that project can be referenced by specifying their fully qualified name: `imported_project_name::target_name`.
    ///
    /// __Example__
    ///
    /// `packages/api/zinoma.yml`:
    ///
    /// ```yaml
    /// name: api
    ///
    /// targets:
    ///   test:
    ///     build: cargo test
    /// ```
    ///
    /// `packages/webapp/zinoma.yml`:
    ///
    /// ```yaml
    /// name: webapp
    ///
    /// targets:
    ///   test:
    ///     build: cargo test
    /// ```
    ///
    /// `./zinoma.yml`:
    ///
    /// ```yaml
    /// imports:
    ///   api: packages/api
    ///   webapp: packages/webapp
    ///
    /// targets:
    ///   test_all:
    ///     dependencies: [api::test, webapp::test]
    /// ```
    ///
    /// In this example, the target `test_all` depend from targets defined in different projects.
    #[serde(default)]
    pub imports: HashMap<String, String>,
}

// TODO Document enum and cases
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, untagged)]
pub enum Target {
    Build {
        /// Lists the targets that must complete successfully before this target can be built.
        ///
        /// It should be an array of strings.
        ///
        /// If any of the dependencies fails to complete, this target will not be executed.
        ///
        /// __Example__
        ///
        /// ```yaml
        /// targets:
        ///   target1: {}
        ///   target2:
        ///     dependencies: [target1]
        ///   target3:
        ///     dependencies: [target2]
        /// ```
        ///
        /// In this example, `target1` must complete successfully before `target2` begins, while `target3` waits for `target2` to complete.
        ///
        /// `zinoma target2` will run sequentially `target1` and `target2`.
        ///
        /// `zinoma target3` will run sequentially `target1`, `target2` and `target3`.
        #[serde(default)]
        dependencies: Vec<String>,

        /// The shell script to run in order to build this target.
        ///
        /// It should be a string which can have one or multiple lines.
        ///
        /// __Example__
        ///
        /// ```yaml
        /// targets:
        ///   create_my_file:
        ///     build: |
        ///       mkdir -p deep/dir
        ///       touch deep/dir/my_file
        /// ```
        ///
        /// In this example, running `zinoma create_my_file` will execute the commands `mkdir -p deep/dir` and `touch deep/dir/my_file` sequentially.
        build: String,

        /// Lists resources identifying the artifacts this target depends on.
        ///
        /// `input` should be an array of [`resources`].
        ///
        /// [`resources`]: enum.InputResource.html#variants
        ///
        /// Specifying a target's `input` enables the incremental build for this target.
        /// This means that, at the time of executing the target, Žinoma will skip its build if its input resources (and [`output`] resources, if any) have not changed since its last successful completion.
        ///
        /// [`output`]: #structfield.output
        ///
        /// __Example__
        ///
        /// ```yaml
        /// targets:
        ///   npm_install:
        ///     input:
        ///       - paths: [package.json, package-lock.json]
        ///     build: npm install
        /// ```
        ///
        /// In this example, running `zinoma npm_install` once will execute `npm install`.
        /// Subsequent runs of `zinoma npm_install` will return immediately — until the content of `package.json` or `package-lock.json` is modified.
        #[serde(default)]
        input: Vec<InputResource>,

        /// Lists resources identifying the artifacts produced by this target.
        ///
        /// It should be an array of [`resources`].
        ///
        /// [`resources`]: enum.OutputResource.html#variants
        ///
        /// If the `--clean` flag is provided to `zinoma`, the files or directories specified in [`output.paths`] will be deleted before running the build flow.
        ///
        /// [`output.paths`]: enum.EnvProbe.html#variant.Paths.field.paths
        ///
        /// The incremental build takes in account the target `output`.
        /// Just like with [`input`], if any of the target output resources were altered since its previous successful execution, the target state will be invalidated and its build will be run again.
        ///
        /// [`input`]: #structfield.input
        ///
        /// __Example__
        ///
        /// ```yaml
        /// targets:
        ///   npm_install:
        ///     input:
        ///       - paths: [package.json, package-lock.json]
        ///     output:
        ///       - paths: [node_modules]
        ///     build: npm install
        /// ```
        ///
        /// In this example, running `zinoma npm_install` will return immediately in case `package.json`, `package-lock.json` and `node_modules` were not modified since the last completion of the target.
        ///
        /// Running `zinoma --clean npm_install` will start by deleting `node_modules`, then will run `npm install`.
        #[serde(default)]
        output: Vec<OutputResource>,
    },

    Service {
        /// Lists the targets that must complete successfully before this target can be built.
        ///
        /// It should be an array of strings.
        ///
        /// If any of the dependencies fails to complete, this target will not be executed.
        ///
        /// __Example__
        ///
        /// ```yaml
        /// targets:
        ///   target1: {}
        ///   target2:
        ///     dependencies: [target1]
        ///   target3:
        ///     dependencies: [target2]
        /// ```
        ///
        /// In this example, `target1` must complete successfully before `target2` begins, while `target3` waits for `target2` to complete.
        ///
        /// `zinoma target2` will run sequentially `target1` and `target2`.
        ///
        /// `zinoma target3` will run sequentially `target1`, `target2` and `target3`.
        #[serde(default)]
        dependencies: Vec<String>,

        /// Shell script starting a long-lasting service to run upon successful build of the target.
        ///
        /// It should be a string.
        ///
        /// This keyword is meant to enable the execution of long-lasting commands, such as servers.
        ///
        /// If the targets to run do not define services, `zinoma` will automatically exit after all builds ran to completion.
        /// On the contrary, if at least one target defines a service, `zinoma` will keep running even after all builds completed, so that the services can remain alive.
        ///
        /// In watch mode (when the `--watch` flag is passed to `zinoma`), services are restarted every time the target `build` runs to completion.
        ///
        /// __Example__
        ///
        /// ```yaml
        /// targets:
        ///   npm_server:
        ///     input:
        ///       - paths: [package.json, index.js]
        ///     build: npm install
        ///     service: npm start
        /// ```
        ///
        /// In this example, `zinoma npm_server` will run `npm install` and then `npm start`.
        service: String,

        /// Lists resources identifying the artifacts this target depends on.
        ///
        /// `input` should be an array of [`resources`].
        ///
        /// [`resources`]: enum.InputResource.html#variants
        ///
        /// Specifying a target's `input` enables the incremental build for this target.
        /// This means that, at the time of executing the target, Žinoma will skip its build if its input resources (and [`output`] resources, if any) have not changed since its last successful completion.
        ///
        /// [`output`]: #structfield.output
        ///
        /// __Example__
        ///
        /// ```yaml
        /// targets:
        ///   npm_install:
        ///     input:
        ///       - paths: [package.json, package-lock.json]
        ///     build: npm install
        /// ```
        ///
        /// In this example, running `zinoma npm_install` once will execute `npm install`.
        /// Subsequent runs of `zinoma npm_install` will return immediately — until the content of `package.json` or `package-lock.json` is modified.
        #[serde(default)]
        input: Vec<InputResource>,
    },

    /// Aggregates other targets.
    ///
    /// e.g.
    ///
    /// ```yaml
    /// targets:
    ///   fmt:
    ///     build: cargo fmt -- --check
    ///   lint:
    ///     build: cargo clippy
    ///   test:
    ///     build: cargo test
    ///   check:
    ///     dependencies: [fmt, lint, test]
    /// ```
    ///
    /// In this example, the target named `check` aggregates the 3 other targets.
    /// `zinoma check` is equivalent to running `zinoma fmt lint test`.
    Aggregate {
        /// Lists the targets that must complete successfully before this target can be built.
        ///
        /// It should be an array of strings.
        ///
        /// If any of the dependencies fails to complete, this target will not be executed.
        ///
        /// __Example__
        ///
        /// ```yaml
        /// targets:
        ///   target1: {}
        ///   target2:
        ///     dependencies: [target1]
        ///   target3:
        ///     dependencies: [target2]
        /// ```
        ///
        /// In this example, `target1` must complete successfully before `target2` begins, while `target3` waits for `target2` to complete.
        ///
        /// `zinoma target2` will run sequentially `target1` and `target2`.
        ///
        /// `zinoma target3` will run sequentially `target1`, `target2` and `target3`.
        dependencies: Vec<String>,
    },
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum InputResource {
    /// Output resources of another target.
    ///
    /// It should be a string with the format `<project_name>::<target_name>.output`.
    /// If the other target is located in the same project, the project name can be skipped.
    /// The `input` would then have this format: `<target_name>.output`.
    ///
    /// When such an input is used:
    ///
    /// - all the output resources of the other target become input resources for this target;
    /// - the other target implicitly becomes a dependency to this target.
    ///
    /// __Example__
    ///
    /// ```yaml
    /// targets:
    ///   node_dependencies:
    ///     input:
    ///       - paths: [package.json, package-lock.json]
    ///     output:
    ///       - paths: [node_modules]
    ///     build: npm install
    ///
    ///   compile:
    ///     input:
    ///       - node_dependencies.output
    ///       - paths: [package.json, tsconfig.json, src]
    ///     output:
    ///       - paths: [dist]
    ///     build: tsc
    ///      
    ///   run:
    ///     input:
    ///       - node_dependencies.output
    ///       - paths: [package.json]
    ///       - compile.output
    ///     service: node dist/index.js
    /// ```
    DependencyOutput(String),
    Paths {
        /// Paths to files or directories.
        ///
        /// It should be an array of strings.
        ///
        /// Each element of the array should be a path to a file or directory.
        ///
        /// __Example__
        ///
        /// ```yaml
        /// targets:
        ///   npm_install:
        ///     input:
        ///       - paths: [package.json, package-lock.json]
        ///     output:
        ///       - paths: [node_modules]
        ///     build: npm install
        /// ```
        paths: Vec<String>,
    },
    CmdStdout {
        /// Shell script whose output identifies the state of a resource.
        ///
        /// It should be a string.
        ///
        /// __Example__
        ///
        /// ```yaml
        /// targets:
        ///   build_docker_image:
        ///     input:
        ///       - paths: [Dockerfile, src]
        ///       - cmd_stdout: 'docker image ls base:latest --format "{{.ID}}"'
        ///     output:
        ///       - cmd_stdout: 'docker image ls webapp:latest --format "{{.ID}}"'
        ///     build: docker build -t webapp .
        /// ```
        cmd_stdout: String,
    },
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum OutputResource {
    Paths {
        /// Paths to files or directories.
        ///
        /// It should be an array of strings.
        ///
        /// Each element of the array should be a path to a file or directory.
        ///
        /// __Example__
        ///
        /// ```yaml
        /// targets:
        ///   npm_install:
        ///     input:
        ///       - paths: [package.json, package-lock.json]
        ///     output:
        ///       - paths: [node_modules]
        ///     build: npm install
        /// ```
        paths: Vec<String>,
    },
    CmdStdout {
        /// Shell script whose output identifies the state of a resource.
        ///
        /// It should be a string.
        ///
        /// __Example__
        ///
        /// ```yaml
        /// targets:
        ///   build_docker_image:
        ///     input:
        ///       - paths: [Dockerfile, src]
        ///       - cmd_stdout: 'docker image ls base:latest --format "{{.ID}}"'
        ///     output:
        ///       - cmd_stdout: 'docker image ls webapp:latest --format "{{.ID}}"'
        ///     build: docker build -t webapp .
        /// ```
        cmd_stdout: String,
    },
}
