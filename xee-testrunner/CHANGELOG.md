# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.6](https://github.com/Paligo/xee/compare/xee-testrunner-v0.1.5...xee-testrunner-v0.1.6) - 2025-08-05

### Other

- Minor change to use https in the github url in Cargo.toml
- Use the sequence serialize function in the test runner. ([#102](https://github.com/Paligo/xee/pull/102))
- Implement fn:trace as a NOOP ([#86](https://github.com/Paligo/xee/pull/86))
- We can now get more tests to pass with the testrunner by enabling xsl:stylesheet. ([#96](https://github.com/Paligo/xee/pull/96))
- Unused.
- Make the xslt test runner do something.
- Make sure we can run the XSLT testrunner again.
- Refactor the test runner so that there's only one URI in sources, not two.
- It ain't pretty and we're not actually running tests yet, but we can at least load up all the tests.
- Attach known dependencies to the language and refactor language.
- Move known dependencies into language module.
- We got some semblance of an XSLT test runner now, though it doesn't actually execute tests yet it at least loads them.
- Further adjustments to try to load the XSLT tests. Not there yet.
- Refactor it so we can start in xpath or xslt language mode.
- Further progress towards the test runner.
- Wire up the basic xslt language stuff.
- First step towards wiring up xslt test case.
- Clippy cleanup.
- Load catalog_ns from context.
- Establish a context for the loading that's used everywhere.
- Make things more consistently context loadable.
- Make it so that static_context_builder in ContextLoadable gets context.
- Simplify the types in the test runner by definining a Language trait with associated types.
- Properly build file scheme uri in testrunner

## [0.1.5](https://github.com/Paligo/xee/compare/xee-testrunner-v0.1.4...xee-testrunner-v0.1.5) - 2025-03-24

### Other

- Add badges.
- Update a whole lot of readmes, linking things.

## [0.1.4](https://github.com/Paligo/xee/compare/xee-testrunner-v0.1.3...xee-testrunner-v0.1.4) - 2025-03-24

### Other

- updated the following local packages: xee-xpath

## [0.1.3](https://github.com/Paligo/xee/compare/xee-testrunner-v0.1.2...xee-testrunner-v0.1.3) - 2025-03-24

### Other

- updated the following local packages: xee-name

## [0.1.2](https://github.com/Paligo/xee/compare/xee-testrunner-v0.1.1...xee-testrunner-v0.1.2) - 2025-03-21

### Other

- release v0.1.2

## [0.1.1](https://github.com/Paligo/xee/releases/tag/xee-testrunner-v0.1.1) - 2025-03-20

Initial public release.