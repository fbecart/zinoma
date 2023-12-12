# Changelog

## TBD

## 0.19.5 (2023-12-12)

Minor version with dependency updates.

## 0.19.4 (2021-03-26)

Minor version with dependency updates.

## 0.19.3 (2021-01-17)

Minor version with dependency updates and code simplification (incl. removal of all unsafe blocks).

## 0.19.2 (2020-10-24)

Minor version with dependency updates.

## 0.19.1 (2020-09-04)

Feature enhancements:

- [FEATURE #67](https://github.com/fbecart/zinoma/issues/67) Filter output files resources by extensions.

Bug fixes:

- [BUG #68](https://github.com/fbecart/zinoma/issues/68) Fix an issue with incremental build and cancellation.

## 0.19.0 (2020-07-24)

Feature enhancements:

- [FEATURE #59](https://github.com/fbecart/zinoma/issues/59) Filter input files resources by extensions.

## 0.18.2 (2020-07-23)

This release was necessary to compensate for failed publications due to [a forced renewal of the crates.io API Token](https://blog.rust-lang.org/2020/07/14/crates-io-security-advisory.html) which broke the release process for 0.18.0 and 0.18.1.

## 0.18.0 (2020-07-23)

This release does not bring any new feature.

Performance improvements:

- [PERF #58](https://github.com/fbecart/zinoma/pull/58) Set up actors model.

## 0.17.0 (2020-07-01)

This release does not bring any new feature. However, it brings a large design overall,
transitioning from a multi-threaded implementation to a single-threaded event loop.

Performance improvements:

- [PERF #56](https://github.com/fbecart/zinoma/pull/56) Switch from `crossbeam` to `async-std`.

## 0.16.0 (2020-06-18)

### BREAKING CHANGES

- Targets should now be one of the following: build target, service or aggregate.
  Each of these target types have distinct fields available (see [details of each variant](https://fbecart.github.io/zinoma/doc/zinoma/config/yaml/schema/enum.Target.html)).

- Services only prevent Žinoma from exiting after a successful build if they are directly requested by the user.

Feature enhancements:

- [FEATURE #48](https://github.com/fbecart/zinoma/issues/48) Services as resources.

## 0.15.3 (2020-06-11)

Žinoma 0.15.3 is a patch release containing small performance improvements.

Feature enhancements:

- [FEATURE #46](https://github.com/fbecart/zinoma/issues/46) Skip file hash computation if timestamps haven't changed.

## 0.15.2 (2020-06-10)

Feature enhancements:

- [FEATURE #41](https://github.com/fbecart/zinoma/issues/41) Distribute Debian packages with a PPA.

## 0.15.1 (2020-06-10)

Bug fixes:

- Fix multiple issues related to resources paths.

## 0.15.0 (2020-06-09)

Feature enhancements:

- [FEATURE #40](https://github.com/fbecart/zinoma/issues/40) Infer target inputs from dependencies.

## 0.14.1 (2020-06-07)

Bug fixes:

- Add titles to schema documentation.

Performance improvements:

- Use `std::process::Command` directly in place of [`run_script`](https://github.com/sagiegurari/run_script).

## 0.14.0 (2020-06-04)

### BREAKING CHANGES

- The way target inputs and outputs are declared was updated.

Instead of:

```yaml
targets:
  my_target:
    input_paths: [my_input]
    output_paths: [my_output]
```

You should now use:

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
  It is now possible to use a shell command as a target input or output.

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
