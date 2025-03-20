# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0](https://github.com/Paligo/xee/releases/tag/xee-name-v0.1.0) - 2025-03-20

### Other

- Update copyright year.
- Update all the licenses to MIT.
- Remove all the apache licenses, MIT only.
- Tweak license text.
- Preparing licenses, attribution, etc.
- make namespaces own their strings, rather than using a lifetime which leaks through everything.
- Retire xee-name::Name in favor of the one from Xot.
- Convert to use Xot's OwnedName.
- Better prefix support.
- An easy way to make a name from a string.
- Add a way to create name from xot name id.
- A very simple pattern registry with lookup by relative name only.
- Inline a few things.
- Add basic readmes for the various sub-crates.
- Factor out naming related stuff into its own module, xee-name.
