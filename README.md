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

- [Homebrew](https://brew.sh/)

```shell script
brew install fbecart/tap/zinoma
```

### Via .deb file (for Debian-based Linux distros)

Download the relevant .deb file from the [releases page](https://github.com/fbecart/zinoma/releases). Then, run:

```shell script
dpkg -i zinoma_*.deb
```

### Via Cargo (for Linux, Windows or macOS)

Prerequisites:

- [Rust toolchain](https://rustup.rs/)

```shell script
cargo install zinoma
```

## Documentation

### YAML syntax for build flows (`zinoma.yml`)

In order to use Žinoma with your project, you need to create a Yaml file named `zinoma.yml`.

The full documentation of the expected schema can be found [on this page](https://fbecart.github.io/zinoma/doc/zinoma/config/yaml/schema/struct.Project.html).

### Command line

```shell script
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

Targets operate on resources (e.g. files), transforming some resources (aka `input`) into other resources (aka `output`).
By looking at the resources declared in the `input` and `output` of your targets,
Žinoma can tell if a target needs to run again, or can be skipped.

Žinoma identifies file updates by looking at their checksum.
The checksums are stored in the `.zinoma` directory, located next to `zinoma.yml`.
This directory should be ignored in your version control.

#### Watch mode (`--watch`)

Žinoma offers a watch mode which can be enabled with the `--watch` option of the command line.

If the watch mode is enabled, `zinoma` will not exit after the build flow completion.
Instead, it will keep an eye open on the targets' `input`'s paths and will re-execute the relevant targets in case filesystem changes are detected.

#### Clean flag (`--clean`)

This flag helps you clean up your build environment.
It will delete files specified in your `targets.<target_name>.outputs.paths` and will reinitialize the targets incremental states.

If provided alone, the `--clean` flag will clean up all targets of your build flow.

When provided along with targets, the `--clean` flag will only run the cleanup on the specified targets and their dependencies.
`zinoma` will then proceed to the execution of these targets.

## Example of configuration

`zinoma.yml`:

```yaml
targets:
  download_dependencies:
    input:
      - paths: [package.json, package-lock.json]
    output:
      - paths: [node_modules]
    build: npm install

  test:
    dependencies: [download_dependencies]
    input:
      - paths: [package.json, node_modules, src, test]
    build: npm test

  lint:
    dependencies: [download_dependencies]
    input:
      - paths: [package.json, node_modules, src, test]
    build: npm run lint

  check:
    dependencies: [test, lint]

  start:
    dependencies: [download_dependencies]
    input:
      - paths: [package.json, src]
    service: exec npm run start

  build:
    dependencies: [check]
    input:
      - paths:
        - Dockerfile
        - package.json
        - package-lock.json
        - src
    output:
      - paths: [lambda.zip]
    build: |
      docker build -t build-my-project:latest .
      docker create -ti --name build-my-project build-my-project:latest bash
      docker cp build-my-project:/var/task/lambda.zip ./
      docker rm -f build-my-project
```

Some example of commands:

- `zinoma check` will ensure the code complies to the test suites and the coding standards.
- `zinoma start --watch` will run the application and restart it whenever the sources are updated.
- `zinoma --clean build` will generate a clean artifact, ready to be deployed.

A fully functional and more advanced example project is available in [fbecart/zinoma-node-example](https://github.com/fbecart/zinoma-node-example).

## Building

Žinoma is written in Rust, so you'll need to grab a [Rust installation](https://rustup.rs/) in order to compile it.

To build Žinoma:

```shell script
$ git clone git@github.com:fbecart/zinoma.git
$ cd zinoma
$ cargo build --release
$ ./target/release/zinoma --version
Žinoma 0.5.1
```

To run the test suite, use:

```shell script
cargo test
```

## Žinoma for the curious

Žinoma is a Lithuanian word. Pronounced it with a stress on the first syllable, which should sound like the _gi_ of _regime_.

In Lithuanian, žinoma has two meanings:

- _of course_, when used as an adverb;
- _known_, when used as an adjective (a reference to the _Not Invented Here Syndrome_).

It is also a recursive acronym for "Žinoma Is NOt MAke!".

## Acknowledgements

This project started as a fork of [Steve Mostovoy's buildy](https://github.com/Stovoy/buildy).

## License

Žinoma is distributed under the terms of the MIT license.

See [LICENSE](LICENSE) for details.
