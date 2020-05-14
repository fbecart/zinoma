# Žinoma
 
Make your build flow incremental

[![Crates.io](https://img.shields.io/crates/v/zinoma.svg)](https://crates.io/crates/zinoma)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Build status](https://github.com/fbecart/zinoma/workflows/ci/badge.svg)](https://github.com/fbecart/zinoma/actions)

---

## Why another build tool?

Non-trivial software projects usually combine multiple technologies, each coming with their specific build tool.
The development workflows on such projects (e.g. checking code validity, deploying a new version) involve multiple commands that need to be executed in a coordinated way.

Running these commands manually is prone to errors, as it is easy to forget commands or to run them in the wrong order.
On the other hand, using a simple script running all of them systematically is unnecessarily slow.

## Introducing Žinoma

Žinoma provides a simple command line to execute your most common build flows in the most efficient way.

In particular, Žinoma provides a mechanism to run the tasks incrementally. This means Žinoma will avoid running repetitive tasks again and again if it determines they can be skipped.

It also handles dependencies between tasks (waiting for the completion of one task before starting another one), and runs tasks in parallel whenever possible.

Finally, Žinoma offers a watch mode, which waits for filesystem updates and re-executes the relevant tasks when source files are updated.

## Installation

### Via Homebrew (for macOS)

Prerequisites:

- Homebrew: https://brew.sh/

```shell script
brew install fbecart/tap/zinoma
```

### Via .deb file (for Debian-based Linux distros)

Download the relevant .deb file from the latest release on https://github.com/fbecart/zinoma/releases. Then, run:

```shell script
dpkg -i zinoma_*.deb
```

### Via Cargo (for Linux, Windows or macOS)

Prerequisites:

- Rust toolchain: https://rustup.rs/

```shell script
cargo install zinoma
```

### From sources (for Linux, Windows or macOS)

Prerequisites:

- Rust toolchain: https://rustup.rs/

```shell script
git clone git@github.com:fbecart/zinoma.git
cd zinoma
cargo install --path .
```

## Documentation

### YAML syntax for build flows (`zinoma.yml`)

In order to use Žinoma with your project, you need to create a file named `zinoma.yml`.
We recommend putting this file in the root directory of your project.

This is the documentation of the format of this file. It assumes prior knowledge of the Yaml format.

#### `targets`

__Required__ A build flow is made of targets. Each target is a unit of work to perform as part of this build flow.

Targets run in parallel by default.
To run targets sequentially, you can define dependencies on other targets using the `targets.<target_name>.dependencies` keyword.

#### `targets.<target_name>`

Each target must have a name.
The key `target_name` is a string and its value is a map of the target's configuration data.

You must replace `<target_name>` with a string that is unique to the `targets` object.
The `<target_name>` must start with an alphanumeric character or `_` and contain only alphanumeric characters, `-`, or `_`.

__Example__

```yaml
targets:
  my_first_target:
  my_second_target:
```

In this example:

- `zinoma my_first_target` will attempt to execute `my_first_target`
- `zinoma my_second_target` will attempt to execute `my_second_target`
- `zinoma my_first_target my_second_target` will run both targets.

#### `targets.<target_name>.dependencies`

Identifies any targets that must complete successfully before this target can run.
It should be an array of strings.

If a target fails, targets that depend on it will not be executed.

__Example__

```yaml
targets:
  target1:
  target2:
    dependencies: [target1]
  target3:
    dependencies: [target2]
```

In this example, `target1` must complete successfully before `target2` begins, while `target3` waits for `target2` to complete.

`zinoma target2` will run sequentially `target1` and `target2`.

`zinoma target3` will run sequentially `target1`, `target2` and `target3`.

#### `targets.<target_name>.build`

This keyword lists the commands to run sequentially in order to build this target. It should be an array of strings, each string representing a command.

__Example__

```yaml
targets:
  create_my_file:
    build:
      - mkdir -p deep/dir
      - touch deep/dir/my_file
```

In this example, running `zinoma create_my_file` will execute the commands `mkdir -p deep/dir` and `touch deep/dir/my_file` sequentially.

#### `targets.<target_name>.input_paths`

Lists the locations of the source files for this target.
`input_paths` should be an array of strings, each representing the path to a file or directory.

The keyword `input_paths` enables the incremental build for this target.
This means that, at the time of executing the target, Žinoma will skip its build if its input files have not changed since its last successful completion.

__Example__

```yaml
targets:
  npm_install:
    input_paths: [ package.json, package-lock.json ]
    build: [ npm install ]
```

In this example, running `zinoma npm_install` once will execute `npm install`.
Subsequent runs of `zinoma npm_install` will return immediately — until the content of `package.json` or `package-lock.json` is modified.

#### `targets.<target_name>.output_paths`

This keyword lists the locations where this target will produce its artifacts.
Similarly to `targets.<target_name>.input_paths`, it should be an array of strings, each representing a path to a file or directory.

If the `--clean` flag is provided to `zinoma`, the files or directories specified in `output_paths` will be deleted before running the build flow.

The incremental build takes in account the `output_paths`.
Just like with `targets.<target_name>.input_paths`, if any of the target output paths were altered since its previous successful execution, the target state will be invalidated and its build will be run again.

__Example__

```yaml
targets:
  npm_install:
    input_paths: [ package.json, package-lock.json ]
    output_paths: [ node_modules ]
    build: [ npm install ]
```

In this example, running `zinoma npm_install` will return immediately in case `package.json`, `package-lock.json` and `node_modules` were not modified since the last completion of the target.

Running `zinoma --clean npm_install` will start by deleting `node_modules`, then will run `npm install`.

#### `targets.<target_name>.service`

Specifies a command to run upon successful build of the target. It should be a string.

This keyword is meant to enable the execution of long-lasting command, such as servers.

Services are only executed in watch mode (when the `--watch` flag is passed to `zinoma`). They are restarted every time the target `build` runs to completion.

__Example__

```yaml
targets:
  npm_server:
    input_paths: [ package.json, index.js ]
    build: [ npm install ]
    service: npm start
```

In this example, `zinoma npm_server --watch` will run `npm install` and then `npm start`.

### Command line

```
USAGE:
    zinoma [FLAGS] [OPTIONS] [--] [TARGETS]...

ARGS:
    <TARGETS>...    Targets to build

FLAGS:
        --clean      Start by cleaning the target outputs
    -h, --help       Prints help information
    -V, --version    Prints version information
    -w, --watch      Enable watch mode: rebuild targets and restart services on file system changes

OPTIONS:
    -p, --project <PROJECT_DIR>    Directory of the project to build (in which 'zinoma.yml' is located)
    -v <verbosity>...              Increases message verbosity
```

### Additional information

#### Incremental build

The incremental build is the core feature of Žinoma.
It is meant to accelerate considerably your development environment,
while simplifying the execution of your most common build flows.

The best way to speed up your build flow is simply to avoid running its commands.
Žinoma helps you do this in a fully automated way.

Targets operate on the file system, transforming some files (aka inputs) into other files (aka outputs).
By looking at the files located in the `input_paths` and `output_paths` of your targets,
Žinoma can tell if a target needs to run, or can be skipped.

Žinoma compares files by computing their checksum.
These checksums are stored in the `.zinoma` directory, located next to `zinoma.yml`.
This directory should be ignored in your version control.

#### Watch mode (`--watch`)

The execution of `zinoma` normally ends as soon as all the specified targets are built.

However, Žinoma also offers a watch mode which can be enabled with the `--watch` option of the command line.
Instead of exiting, Žinoma will keep an eye open on the targets' `input_paths`,
and will re-execute the relevant targets in case filesystem changes are detected.

When watch mode is enabled, Žinoma also runs the services of the built targets.
A service will be restarted every time its target's build completes.

#### Clean flag (`--clean`)

This flag helps you clean up your build environment.
It will delete files specified in your `targets.<target_name>.output_paths` and will reinitialize the targets incremental states.

If provided alone, the `--clean` flag will clean up all targets of your build flow.

When provided along with targets, the `--clean` flag will only run the cleanup on the specified targets and their dependencies.
`zinoma` will then proceed to the execution of these targets.

## Example of configuration

`zinoma.yml`:

```yaml
targets:
  download_dependencies:
    input_paths: [ package.json, package-lock.json ]
    output_paths: [ node_modules ]
    build: [ npm install ]

  test:
    dependencies: [ download_dependencies ]
    input_paths: [ package.json, node_modules, src, test ]
    build: [ npm test ]

  lint:
    dependencies: [ download_dependencies ]
    input_paths: [ package.json, node_modules, src, test ]
    build: [ npm run lint ]

  check:
    dependencies: [ test, lint ]

  start:
    dependencies: [ download_dependencies ]
    input_paths: [ package.json, src ]
    service: npm run start

  build:
    dependencies: [ check ]
    input_paths:
      - Dockerfile
      - package.json
      - package-lock.json
      - src
    output_paths: [ lambda.zip ]
    build:
      - docker build -t build-my-project:latest .
      - docker create -ti --name build-my-project build-my-project:latest bash
      - docker cp build-my-project:/var/task/lambda.zip ./
      - docker rm -f build-my-project
```

Some example of commands:

- `zinoma check` will ensure the code complies to the test suites and the coding standards.
- `zinoma start --watch` will run the application and restart it whenever the sources are updated.
- `zinoma --clean build` will generate a clean artifact, ready to be deployed.

## Project roadmap

- [x] Execute targets in parallel
- [x] Handle dependencies between targets (and detect cyclic dependencies)
- [x] Check input paths and output paths in incremental build
- [x] Watch mode
- [x] Clean command
- [x] Basic auto-completion
- [ ] Auto-complete target names
- [ ] Accept configuration split in multiple files (would be especially useful for repositories containing multiple projects)
- [ ] Accept scripted configuration
- [ ] Provide a way to import/extend configuration templates

## Žinoma for the curious

Žinoma is a Lituanian word. Pronounced it with a stress on the first syllable, which should sound like the _gi_ of _regime_.

In Lithuanian, žinoma has two meanings:

- _of course_, when used as an adverb;
- _known_, when used as an adjective (a reference to the _Not Invented Here Syndrome_).

It is also a recursive acronym for "Žinoma Is NOt MAke!".

## Acknowledgements

This project started as a fork of https://github.com/Stovoy/buildy.

## License

Žinoma is distributed under the terms of the MIT license.

See [LICENSE](LICENSE) for details.
