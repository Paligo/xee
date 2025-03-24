# xee-testrunner
[![Crates.io](https://img.shields.io/crates/v/xee-testrunner.svg)](https://crates.io/crates/xee-testrunner)
[![Documentation](https://docs.rs/xee-testrunner/badge.svg)](https://docs.rs/xee-testrunner)


This is a test runner that can run the XPath conformance test suite in the [Xee
project](https://github.com/Paligo/xee). Work on enabling the XSLT conformance
test suite is in progress.

We have added both the [XPath conformance test
suite](https://github.com/w3c/qt3tests) and the [XSLT conformance test
suite](https://github.com/w3c/xslt30-test/) under the `vendor` directory of
this project, as `vendor/xpath-tests` and `vendor/xslt-tests`.

The test runner will in the future detect automatically whether you're running
the XPath or XSLT tests, and adjust its behavior accordingly.

You can install this test runner using `cargo install`, but that makes it
difficult to check immediately against changes in the Xee codebase itself,
which is what is it for. The instructions here describe how to run the tests
against the current state of the Xee project. We compile in `--release` mode as
otherwise running the tests takes a long time.

To check against regressions, run:

```
cargo run --release  -- check ../vendor/xpath-tests/
```

or (in the future)

```
cargo run --release  -- check ../vendor/xslt-tests/
```

To run all tests (for XPath or XSLT):

```
cargo run --release  -- all ../vendor/xpath-tests/
cargo run --release  -- all ../vendor/xslt-tests/
```

You can run the tests and update the regression filter accordingly:

```
cargo run --release -- update ../vendor/xpath-tests/
```

See the [hacking guide](https://github.com/Paligo/xee/blob/main/hacking.md) of
the [Xee project](https://github.com/Paligo/xee) to see how this can be used
during development.

This is a development tooling crate of the [Xee
project](https://github.com/Paligo/xee). For the API entry point see
[`xee-xpath`](https://docs.rs/xee-xpath/latest/xee_xpath/). For the `xee`
commandline tool, download a
[release](https://github.com/Paligo/xee/releases/).

## More Xee

[Xee homepage](https://github.com/Paligo/xee)

## Credits

This project was made possible by the generous support of
[Paligo](https://paligo.net/).