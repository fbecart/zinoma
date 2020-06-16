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
    /// [`Targets`] represent commands and scripts to execute in your build flow.
    ///
    /// [`Targets`]: struct.Target.html
    ///
    /// Targets run in parallel by default.
    /// To force targets to run sequentially, you can define [`dependencies`] on other targets.
    ///
    /// [`dependencies`]: enum.Target.html#variant.Build.field.dependencies
    ///
    /// Each target must have a unique name inside the project.
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
    /// - `zinoma speak_cow speak_dog` will print both `Moo` and `Woof!` (not necessarily in this order)
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

/// A target is a command or a set of commands to run as part of your build flow.
///
/// Targets run in parallel by default.
/// To force targets to run sequentially, you can define [`dependencies`] on other targets.
///
/// [`dependencies`]: struct.Dependencies.html
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, untagged)]
pub enum Target {
    /// A build target represents a shell script to run as part of your build flow.
    ///
    /// This build script is expected to eventually complete,
    /// as opposed to the run script of a [`service`] target.
    ///
    /// [`service`]: #variant.Service.field.service
    Build {
        /// Dependencies of the target.
        #[serde(default)]
        dependencies: Dependencies,

        /// The shell script to run in order to build this target.
        ///
        /// It should be a string. This string can be multi-line, in case of scripts with multiple commands.
        ///
        /// __Example__
        ///
        /// ```yaml
        /// targets:
        ///   create_file_deep:
        ///     build: |
        ///       mkdir -p deep/dir
        ///       touch deep/dir/file
        ///     output:
        ///       - paths: [deep/dir/file]
        /// ```
        ///
        /// In this example, running `zinoma create_file_deep` will execute the commands `mkdir -p deep/dir` and `touch deep/dir/my_file` sequentially.
        build: String,

        /// Input resources of the target.
        #[serde(default)]
        input: InputResources,

        /// Output resources of the target.
        #[serde(default)]
        output: OutputResources,
    },

    /// service targets are useful to run scripts that do not complete.
    /// They enable the execution of long-lasting commands, such as servers.
    Service {
        /// Dependencies of the target.
        #[serde(default)]
        dependencies: Dependencies,

        /// Shell script starting a long-lasting service.
        ///
        /// It should be a string.
        ///
        /// If `zinoma` has no service target to run, it will automatically exit after all build targets ran to completion.
        /// On the contrary, if there is at least one service target to run,
        /// `zinoma` will keep running even after all build targets completed, so that the services can remain alive.
        ///
        /// In watch mode (when the `--watch` flag is passed to `zinoma`), services are restarted when the relevant paths are modified.
        ///
        /// __Example__
        ///
        /// ```yaml
        /// targets:
        ///   npm_server:
        ///     input:
        ///       - paths: [package.json, index.js]
        ///     service: npm start
        /// ```
        ///
        /// In this example, `zinoma npm_server --watch` will run `npm start`,
        /// and will restart this process every time `package.json` or `index.js` are updated.
        service: String,

        /// Input resources of the target.
        #[serde(default)]
        input: InputResources,
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
        /// Dependencies of the target.
        dependencies: Dependencies,
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
        /// Each element of the array should be a path to a file or directory.
        ///
        /// If the `--clean` flag is provided to `zinoma`, the files or directories specified in `paths` will be deleted before running the build flow.
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
        /// In this example, as the target specifies an `input`, `zinoma npm_install` is incremental.
        /// The script `npm install` will be skipped until `package.json`, `package-lock.json` or `node_modules` are modified.
        ///
        /// Additionally:
        ///
        /// - the command `zinoma --clean` will delete `node_modules`;
        /// - the command `zinoma --clean npm_install` will delete `node_modules`, then run `npm install`.
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

/// List of [`targets`] that must complete successfully before this target can be built.
///
/// [`targets`]: enum.Target.html
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
#[derive(Debug, Default, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Dependencies(#[serde(default)] pub Vec<String>);

/// List of artifacts that this target depends on.
///
/// `input` should be an array of [`resources`].
///
/// [`resources`]: enum.InputResource.html
///
/// Specifying a target's `input` enables the incremental build for this target.
/// This means that, at the time of executing the target, Žinoma will skip its build if its input resources (and [`output`] resources, if any) have not changed since its last successful completion.
///
/// [`output`]: struct.OutputResources.html
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
#[derive(Debug, Default, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct InputResources(#[serde(default)] pub Vec<InputResource>);

/// List of artifacts produced by this target.
///
/// It should be an array of [`resources`].
///
/// [`resources`]: enum.OutputResource.html
///
/// The incremental build takes in account the target `output`.
/// Just like with [`input`], if any of the target output resources were altered since its previous successful execution, the target state will be invalidated and its build will be run again.
///
/// [`input`]: struct.InputResources.html
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
#[derive(Debug, Default, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct OutputResources(#[serde(default)] pub Vec<OutputResource>);
