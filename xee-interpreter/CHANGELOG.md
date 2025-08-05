# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.2.0](https://github.com/Paligo/xee/compare/xee-interpreter-v0.1.5...xee-interpreter-v0.2.0) - 2025-08-05

### Added

- Adds static default namespaces

### Other

- Printing application error description when displaying error message
- Minor change to use https in the github url in Cargo.toml
- Do not depend on strum_macros
- Use the sequence serialize function in the test runner. ([#102](https://github.com/Paligo/xee/pull/102))
- Add "xee xlst" subcommand for CLI
- More default template rules. ([#100](https://github.com/Paligo/xee/pull/100))
- Implement fn:trace as a NOOP ([#86](https://github.com/Paligo/xee/pull/86))
- We can now get more tests to pass with the testrunner by enabling xsl:stylesheet. ([#96](https://github.com/Paligo/xee/pull/96))
- Just in time document order ([#95](https://github.com/Paligo/xee/pull/95))
- Implement fn:random-number-generator.
- Define concat with arity 99
- Use arithmetic casting for op:numeric-subtract

## [0.1.5](https://github.com/Paligo/xee/compare/xee-interpreter-v0.1.4...xee-interpreter-v0.1.5) - 2025-03-24

### Other

- Badges.
- Update a whole lot of readmes, linking things.
- Credits and more links

## [0.1.4](https://github.com/Paligo/xee/compare/xee-interpreter-v0.1.3...xee-interpreter-v0.1.4) - 2025-03-24

### Other

- updated the following local packages: xee-xpath-macros

## [0.1.3](https://github.com/Paligo/xee/compare/xee-interpreter-v0.1.2...xee-interpreter-v0.1.3) - 2025-03-24

### Other

- updated the following local packages: xee-name

## [0.1.2](https://github.com/Paligo/xee/compare/xee-interpreter-v0.1.1...xee-interpreter-v0.1.2) - 2025-03-21

### Fixed

- Fix underflow and overflow errors in array access.
- Rewrite substring logic to avoid underflow/overflow issues. Also fix
  `fn-substring-22` along the way.

## [0.1.1](https://github.com/Paligo/xee/releases/tag/xee-interpreter-v0.1.1) - 2025-03-20

Initial public release.
