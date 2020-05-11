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

### Via Homebrew (for macOS only)

Prerequisites:

- Homebrew: https://brew.sh/

```shell script
brew install fbecart/zinoma/zinoma
```

### Via Cargo (for Linux, Windows or macOS)

Prerequisites:

- Rust toolchain: https://rustup.rs/

```shell script
cargo install zinoma
```

### Setup auto-completion for Z shell (Zsh)

To set up Žinoma auto-completion for Zsh, put the completion script in one of the paths in your `$fpath`. For instance:

```shell script
zinoma --generate-zsh-completion > $HOME/.zfunc/_zinoma
```

You should update this script when you install a new version of Žinoma.

## Documentation

### Project configuration file

This documentation assumes prior knowledge of the Yaml format. If you're not familiar with Yaml, you should first get accustomed to the basics of its syntax.

Configure your project with a Yaml file called `zinoma.yml` at the root of your project.

```yaml
# List the targets (aka tasks) of your project workflow
targets:
  # Declare target "npm-install"
  npm-install:
    # List the locations of the sources for this target (optional)
    input_paths: [ package.json, package-lock.json ]

    # List locations where this target will produce its artifacts (optional)
    output_paths: [ node_modules ]

    # List commands to run sequentially in order to build this target (optional)
    build: [ npm install ]

  # Declare target "start-server"
  start-server:
    # List other target this target depends on (optional)
    # This means "start-server" will only be executed upon a successful build of "npm-install".
    dependencies: [ npm-install ]

    # State the command which starts this service (optional)
    # A service is a long-lasting command, such as a server.
    # It will only be executed in watch mode, upon a successful build (or rebuild) of the same target.
    service: npm start
```

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

The build of a target will be skipped if the `input_paths` and `output_paths` have been left untouched since its last successful execution.

Žinoma keeps track of the build state in a directory named `.zinoma`, located at the root of the project (where `zinoma.yml` is located). This directory should be ignored in version control.

#### Watch mode

The execution of `zinoma` will normally end as soon as all the specified targets were built.

However, Žinoma also offers a watch mode which can be enabled with the `--watch` option of the command line.
When the watch mode is enabled, Žinoma also runs the services of the built targets, and does not exit.
Žinoma will then keep an eye open on the `input_paths`, and will re-execute the relevant targets in case filesystem changes are detected.

## Example of configuration

`zinoma.yml`:

```yaml
targets:
  download-dependencies:
    input_paths: [ package.json, package-lock.json ]
    output_paths: [ node_modules ]
    build: [ npm install ]

  test:
    dependencies: [ download-dependencies ]
    input_paths: [ package.json, node_modules, src, test ]
    build: [ npm test ]

  lint:
    dependencies: [ download-dependencies ]
    input_paths: [ package.json, node_modules, src, test ]
    build: [ npm run lint ]

  check:
    dependencies: [ test, lint ]

  start:
    dependencies: [ download-dependencies ]
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

## Roadmap

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

Žinoma is a Lituanian word. It should be pronounced _\[zhee-no-ma]_, with a stress on the first syllabus.

In Lithuanian, the word has two meanings:

- _of course_, when used as an adverb;
- _known_, when used as an adjective (a reference to the _Not Invented Here Syndrome_).

It is also a recursive acronym for "Žinoma Is NOt MAke!".

## Acknowledgements

This project started as a fork of https://github.com/Stovoy/buildy

## License

Žinoma is distributed under the terms of the MIT license.

See [LICENSE](LICENSE) for details.
