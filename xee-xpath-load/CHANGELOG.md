# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0](https://github.com/Paligo/xee/releases/tag/xee-xpath-load-v0.1.0) - 2025-03-20

### Other

- Update copyright year.
- Update all the licenses to MIT.
- Remove all the apache licenses, MIT only.
- Make insta a single workspace dependency so it's easier to upgrade.
- Tweak license text.
- Preparing licenses, attribution, etc.
- Port the tests from the xee-xpath-compiler to the xee-xpath API.
- Retire Uri to indicate documents in favor if iri-string based approach.
- Realized that session wasn't pulling its weight, so replace it with documents.
- Further modify xee-xpath so that we can use it fully in the test runner.
- Session is created from documents alone now.
- The load system had a crazy lifetime story. Massively simplify lifetimes.
- A lot of work to make the API cleaner.
- Isolated static context and reconstruct dynamic context each time we run.
- Improve documents API by pushing most of it down into the interpreter.
- Remove old query system. it's in xee-xpath now.
- We use the new queries infastructure to do the work for the testrunner.
- Modify how variables are passed.
- Update Xot version.
- Make it so that the various iterators return an error early if the data is absent.
- Rename xee-xpath to xee-xpath-compiler so we can better hide information from the public APIs.
- We try to move towards a DynamicContext we can live with.
- Dynamic context owns static context now.
- Make it possible to clone a map query.
- Rework dynamic context so that it owns documents, which makes the API we want in the load functionality feasible.
- More renaming, calling the hook method "load" instead of query.
- Use xee-xpath-load from xee-testrunner.
- Move over the load from xml, file traits too.
- Use anyhow for error handling, as this is the simplest.
- Start to extract xpath load functionality from xee-xpath.
