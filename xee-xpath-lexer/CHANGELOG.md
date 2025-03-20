# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0](https://github.com/Paligo/xee/releases/tag/xee-xpath-lexer-v0.1.0) - 2025-03-20

### Other

- Update copyright year.
- Update all the licenses to MIT.
- Remove all the apache licenses, MIT only.
- Upgrade logos too.
- Upgrade itertools.
- Upgrade ron.
- Upgrade some more dependencies and unify rust_decimal_macros.
- Make insta a single workspace dependency so it's easier to upgrade.
- Update to newer version of Rust and do some clippy work.
- Tweak license text.
- Preparing licenses, attribution, etc.
- Fix comment parsing.
- Add a few cases I'm debugging to make sure the tokenizer behaves as expected.
- mapmap is indeed a ncname
- Modify so that xee-xpath-ast uses xee-xpath-lexer
- Prefixes can also be reserved names.
- We can handle reserved names as function names in prefixed qnames now.
- Clippy.
- Only a single implementation of symbol type, in its own module.
- Rename delimination2 to delimination
- Get rid of original iterator.
- We turn out not to need comment into whitespace or whitespace collapse to make it work.
- Some cleanups.
- Rename.
- A few more special cases.
- Plug the new lexer into the old tests.
- Unused import.
- Simplified version of delimination rules.
- Further test cleanup.
- Unused whitespace.
- More test cleanup.
- Cleaning test infrastructure up a bit before proceeding.
- Debugging print.
- Collapse whitespace tests and fixes.
- Comment processing logic.
- nested comment.
- Start of comment replacing with whitespace code.
- Add various tests for the post-processing scenarios.
- Explicit whitespace iterator improvements.
- add explanation
- Verifying that it handles stuff following.
- prefixed qname combination in lexer.
- Some lexer experiments.
- Simplify match logic a bit.
- Extract out delimination logic.
- extracting lexer.
