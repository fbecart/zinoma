# Changelog

## TBD

### BREAKING CHANGES

- The way target inputs and outputs are declared was updated.

From:

```yaml
targets:
  my_target:
    input_paths: [my_input]
    output_paths: [my_output]
```

You should now have:

```yaml
targets:
  my_target:
    input:
      - paths: [my_input]
    output:
      - paths: [my_output]
```

Feature enhancements:

- [FEATURE #16](https://github.com/fbecart/zinoma/issues/16) Accept more options for target inputs and outputs.

It is now possible to use a command as a command input or output.

__Example__ _(from [zinoma-node-example](https://github.com/fbecart/zinoma-node-example/blob/master/zinoma.yml))_

```yaml
targets:
  e2e_docker_run:
    dependencies:
      [backend::docker_build, webapp::docker_build, e2e::docker_build]
    input:
      - paths: [docker-compose.e2e.yml, docker-compose.yml]
      - cmd_stdout: 'docker image ls todos/backend:latest --format "{{.ID}}"'
      - cmd_stdout: 'docker image ls todos/webapp:latest --format "{{.ID}}"'
      - cmd_stdout: 'docker image ls todos/e2e:latest --format "{{.ID}}"'
    output:
      - paths: [target]
    build: docker-compose -f docker-compose.yml -f docker-compose.e2e.yml up --exit-code-from todos-e2e
```

## 0.13.0 (2020-06-01)

Feature enhancements:

- [FEATURE #36](https://github.com/fbecart/zinoma/issues/36) Directory `.zinoma` should be ignored in watchers and incremental builds.

Bug fixes:

- Žinoma would not work on Windows due to [`Path::canonicalize` returning an UNC path](https://github.com/rust-lang/rust/issues/42869#issuecomment-346362633).

## 0.12.0 (2020-05-30)

### BREAKING CHANGES

- Imported projects need to have a name defined.
  This name becomes a key to the `imports` object.
  Imported targets should be referred to with their fully qualified name: `project_name::target_name`.

Feature enhancements:

- [FEATURE #27](https://github.com/fbecart/zinoma/issues/27) Each project should be a namespace for target names.

## 0.11.0 (2020-05-28)

### BREAKING CHANGES

- The Yaml parser now denies additional properties.

Feature enhancements:

- [FEATURE #31](https://github.com/fbecart/zinoma/issues/31) Document Yaml configuration schema for improved IDE experience.

## 0.10.0 (2020-05-27)

Feature enhancements:

- [FEATURE #31](https://github.com/fbecart/zinoma/issues/31) Generate schema for the syntax of the Yaml configuration.

## 0.9.0 (2020-05-27)

Performance improvements:

- [PERF #13](https://github.com/fbecart/zinoma/issues/13) Use Jemalloc as the global allocator.

## 0.8.1 (2020-05-27)

This patch version fixes a bug, improves documentation (configuration examples) and introduces the first benchmarks.

Bug fixes:

- An input or output path missing from the filesystem causes Žinoma to fail.

## 0.8.0 (2020-05-21)

Feature enhancements:

- [FEATURE #8](https://github.com/fbecart/zinoma/issues/8): Split build flow configuration in multiple projects.

## 0.7.0 (2020-05-19)

### BREAKING CHANGES

- `targets.<target_name>.service` now affects the execution of Žinoma, even when `--watch` is not provided.

Feature enhancements:

- [FEATURE #9](https://github.com/fbecart/zinoma/issues/14): Services should run even without the `--watch` mode.

## 0.6.0 (2020-05-14)

If Žinoma was to follow Semver, this would absolutely be a major release. But until then, let's enjoy having a version number below 1.

### BREAKING CHANGES

- The format of `targets.<target_name>.build` has changed. Instead of accepting an array of commands, this keyword now accepts a multi-line string.

Feature enhancements:

- [FEATURE #14](https://github.com/fbecart/zinoma/issues/14): Accept multi-line build scripts for targets' `build` and `run` keywords.

## 0.5.1 (2020-05-13)

Žinoma 0.5.1 is a patch release created to fix the released _.deb_ artifacts.

## 0.5.0 (2020-05-13)

Žinoma 0.4.0 is a minor version release which introduces _.deb_ artifacts for Debian-based Linux distributions.

## 0.4.\[1-3] (2020-05-12)

Žinoma 0.4.1 to 0.4.3 are patch releases created to test the improved release automation.

## 0.4.0 (2020-05-12)

Žinoma 0.4.0 is a minor version release which includes the addition of autocompletion scripts for Bash, Fish and Powershell (on top of Zsh).
These completion scripts will be added to the Homebrew formula.

## 0.3.0 (2020-05-11)

Žinoma 0.3.0 is a minor version release which introduces binaries for Linux.

## 0.2.0 (2020-05-11)

Žinoma 0.2.0 is the first minor version release. It introduces binaries for macOS X. Its primary intent is to make the installation via Homebrew possible.
