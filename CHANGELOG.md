# Changelog

We will document all notable changes to this project in this file.

We use the [Keep a Changelog](https://keepachangelog.com/en/1.0.0/) format,
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.8] - 2020-05-03

### Added

- `"when"` for conditional jobs
- new [facts](./docs/template.md) for OS-detection
- document available template expression values / [facts](./docs/template.md)
- `{{ has_executable(foo) }}` template function to check for executables

### Changed

- also check for tuning/main.toml in ~/.dotfiles

### Fixed

- escape path expressions so Windows paths are valid TOML (#17)

## [0.1.7] - 2020-04-06

### Added

- colorized output for job status
- generate friendlier names for command jobs
- generate friendlier names for file jobs
- internally centralise handling of common fields like `"name"`

## [0.1.6] - 2020-03-13

### Added

- links to wiki documentation in README
- read config file from default location
- `"needs"` for inter-job dependencies
- command: job type to run commands
- concurrent job runner using 2 threads
- file: job type to manipulate files
- support `{{ home_dir }}` and other expressions in template

## [0.1.5] - 2019-08-16

### Fixed

- ci: cannot use `contains()` with array

## [0.1.4] - 2019-08-16

### Fixed

- ci: tweak handling of "release" GitHub Action

## [0.1.3] - 2019-08-16

### Fixed

- add missing description and license metadata

## [0.1.2] - 2019-08-16

### Added

- ci: debug GitHub Actions workflow

## [0.1.1] - 2019-08-16

### Fixed

- ci: fix broken GitHub Actions `if`

## [0.1.0] - 2019-08-16

### Added

- initial (non-functional) release to crates.io
