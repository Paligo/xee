# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0](https://github.com/Paligo/xee/releases/tag/xee-xslt-compiler-v0.1.0) - 2025-03-20

### Other

- Update copyright year.
- Update all the licenses to MIT.
- Remove all the apache licenses, MIT only.
- Make insta a single workspace dependency so it's easier to upgrade.
- Tweak license text.
- Preparing licenses, attribution, etc.
- Move things from trait onto Sequence.
- Rewrite to consolidated sequence.
- Add the ability to add documents without a URI.
- Retire Uri to indicate documents in favor if iri-string based approach.
- Make things more private, and expose as methods instead.
- Clippy.
- The dynamic context builder should be built from the program.
- Let program own static context.
- make namespaces own their strings, rather than using a lifetime which leaks through everything.
- Lots more clippy.
- Introduce DocumentsRef and StaticContextRef to wrap Rc away from public API.
- Isolated static context and reconstruct dynamic context each time we run.
- Add the context item to the dynamic context too.
- Use a builder everywhere for dynamic context.
- Passing variables separately from the DynamicContext turns out to be misguided.
- Further API cleanups.
- Improve documents API by pushing most of it down into the interpreter.
- Modify how variables are passed.
- Make it so that the various iterators return an error early if the data is absent.
- Also rename xee-xslt to xee-xslt-compiler.
