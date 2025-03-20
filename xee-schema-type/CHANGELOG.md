# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0](https://github.com/Paligo/xee/releases/tag/xee-schema-type-v0.1.0) - 2025-03-20

### Other

- Update copyright year.
- Update all the licenses to MIT.
- Remove all the apache licenses, MIT only.
- Make insta a single workspace dependency so it's easier to upgrade.
- Inline a bit more. Doesn't seem to make much of a difference.
- Should be a faster derives from.
- Tweak license text.
- Preparing licenses, attribution, etc.
- Convert to use Xot's OwnedName.
- Add basic readmes for the various sub-crates.
- Rename old xee-xpath to xee-interpreter.
- Clean up imports to use indirect imports only.
- Nicer Variables API.
- Parse atomic type directly into the Xs type when we construct the AST.
- Work towards date-time functions.
- years-from-duration support
- Support xs:numeric enough to implement fn:abs
- Speculative implementation of support for more atomic types.
- Introduce xs:AnyURI
- Instead of a lot of enum for various integer types, store it all as an ibig
- Extend xee-schema-types with all atomic types.
- Use xee-schema-type to generate rust type name.
- Introduce a base numeric type system and rewrite in terms of that.
- Use ibig to store integer, to be closer to spec.
- Better IR generation for cast and castable.
- Comparison now obeys the casting rules and subtypes for integers work.
- Simpler module name.
- Simplification - we use enums instead of type names.
- Rename for consistency.
- Rename xee-schematype to xee-schema-type for consistency.
