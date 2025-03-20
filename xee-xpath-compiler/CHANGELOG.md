# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0](https://github.com/Paligo/xee/releases/tag/xee-xpath-compiler-v0.1.0) - 2025-03-20

### Other

- Update copyright year.
- Update all the licenses to MIT.
- Remove all the apache licenses, MIT only.
- Make insta a single workspace dependency so it's easier to upgrade.
- Move id-related function into a separate module.
- Tweak license text.
- Preparing licenses, attribution, etc.
- Adjust the text as we don't do execution anymore.
- We can now remove the old evaluation code in xee-xpath-compiler.
- Clean up more old cruft that isn't pulling its weight.
- Also move over arithmetic tests.
- Port the tests from the xee-xpath-compiler to the xee-xpath API.
- Non-consecutive range test.
- Check that ranges are indeed combined correctly.
- A more efficient representation of ranges.
- Fix a bug where we really need absent information in order to create the proper closure.
- Rewrite to consolidated sequence.
- Reject namespace axis as a compiler error.
- Implement parse-xml-fragment.
- Add fn:parse-xml
- Add the ability to add documents without a URI.
- Retire Uri to indicate documents in favor if iri-string based approach.
- Upgrade to xot 0.27. Also update a failing snapshot test.
- Refactor the way collations work. This fixes 2 tests in the big test suite.
- Make things more private, and expose as methods instead.
- Clippy.
- The dynamic context builder should be built from the program.
- Let program own static context.
- make namespaces own their strings, rather than using a lifetime which leaks through everything.
- More clippy.
- Lots more clippy.
- Introduce DocumentsRef and StaticContextRef to wrap Rc away from public API.
- Isolated static context and reconstruct dynamic context each time we run.
- Add the context item to the dynamic context too.
- Use a builder everywhere for dynamic context.
- Passing variables separately from the DynamicContext turns out to be misguided.
- A static context builder so it's easier to create them.
- Further API cleanups.
- Improve documents API by pushing most of it down into the interpreter.
- Modify how variables are passed.
- More clippy and documentation work.
- Make span optional in SpannedError. This makes it a bit more convenient to use it in the outer layer.
- Move high level stuff into xee-xpath.
- Rename xee-xpath to xee-xpath-compiler so we can better hide information from the public APIs.
