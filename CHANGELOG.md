# Changelog

## TBD

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
