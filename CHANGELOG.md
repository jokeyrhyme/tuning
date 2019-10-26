# Changelog

We will document all notable changes to this project in this file.

We use the [Keep a Changelog](https://keepachangelog.com/en/1.0.0/) format,
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- links to wiki documentation in README
- read config file from default location
- `"needs"` for inter-job dependencies
- command: job type to run commands
- concurrent job runner using 2 threads
- file: job type to manipulate files

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
