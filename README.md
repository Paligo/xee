# Xee

XML Execution Engine written in Rust.

It's been made possible by the generous support of
[Paligo](https://paligo.net/). See 

## What is Xee?

The Xee project contains the following:

- An almost complete [XPath 3.1](https://www.w3.org/TR/xpath-31/)
  implementation. Use it via the `xee-xpath` crate.

- A command line tool, `xee`, which can be used to load XML documents, issue
  XPath expressions against them, including in a REPL, and pretty-print XML
  documents. It's intended to become a Swiss Army knife CLI for XML.

- An incomplete [XSLT 3.0](https://www.w3.org/TR/xslt-30/) implementation. At
  the time of writing a lot of XSLT is yet to be implemented, but there is a
  strong foundation. Please contribute!

## Obtaining the `xee` commandline tool

You can download a pre-built binary from
[releases](https://github.com/Paligo/xee/releases).

Alternatively you can compile it yourself. First you need to make sure you have
the [Rust toolchain installed](https://www.rust-lang.org/tools/install).

Then:

```
git clone https://github.com/Paligo/xee.git
cd xee
cargo install --path .
```

Note that you need a recent stable version of Rust; `rustup update` should get
you one.

## Executing XPath expressions from Rust

You can also use the [provided `xee-xpath`
API](https://docs.rs/xee-xpath/latest/xee_xpath/) to execute XPath expressions
from within Rust.

## Executing XPath expressions from PHP

A prototype PHP binding library is available as
[`xee-php`](https://github.com/Paligo/xee-php)

## How is Xee implemented?

Both XPath and XSLT are supported by the same bytecode interpreter. Compilation
machinery exists to transform XPath and XSLT into bytecode, which the
interpreter can then execute.

The XPath functions in [the XPath standard function
library](https://www.w3.org/TR/xpath-functions-31/) are implemented in Rust,
using a Rust binding system. The implementation is extensive but not everything
is implemented yet; see
[conformance](https://github.com/Paligo/xee/tree/main/conformance) for more.

An affiliated project is [`regexml`](https://github.com/Paligo/regexml), which
contains an XML Schema and XPath compatible version of regular expressions for
Rust. 

[`xee-php`](https://github.com/Paligo/xee-php) is the start of PHP bindings for
Xee.

## Testing

This project undergoes extensive automated testing using using specific
developer tests executed using `cargo test` as well as using the
`xee-testrunner` infrastructure for running conformance tests. This is run in
CI for PRS, and you can also run them locally. See [`hacking`](hacking.md) for
more information.

## What's missing?

The XML world is very heavily specified, so this project implements detailed
and very extensive specifications.

Here is a brief description of the state of specification conformance in this
project. Contributions are encouraged!

### XPath

- While much of it is covered, some of the functions in [XPath and XQuery
  Functions and Operators 3.1](https://www.w3.org/TR/xpath-functions-31/) are
  not yet implemented. Contributions are welcome!

- Of the 21859 tests in the QT3 test suite (vendored into `vendor/xpath-tests`)
  that match the features we support (so excluding XQuery tests), we support
  over have 20130 at the time of writing. The failures are mostly due to
  missing library implementation.

- XMLSchema support. While the basic `xs:*` data types as defined by XML Schema
  are implemented, deep XML Schema integration does not exist.

- The Rust binding system for XPath can only be used to implement standard
  library functions - support for extension functions needs to be created.

- We do believe Xee provides a solid basis for optimization work but we've only
  scratched the surface of what's possible.

### XSLT

- The basic infrastructure of compiling XSLT to bytecode has been implemented,
  including a full XSLT AST, basic control flow, and template selection, but
  much XSLT functionality yet remains to be implemented.

## Architecture

XPath gets lexed into tokens using a lexer (logos) in
[`xee-xpath-lexer`](xee-xpath-lexer). This is then parsed into an XPath AST
(abstract syntax tree) by [`xee-xpath-ast`](`xee-xpath-ast`). This AST is then
compiled down into a specialized IR (intermediate representation) by
[`xee-xpath-compiler`](`xee-xpath-compiler`). This IR is then compiled down
into a bytecode by [`xee-ir`](`xee-ir`). This bytecode is executed using the
interpreter implemented in [`xee-interpreter`](`xee-interpreter`) which also
implements the standard library functions for XPath.

An XPath API is exposed for use by others in [`xee-xpath`](xee-xpath) ([API
docs](https://docs.rs/xee-xpath/latest/xee_xpath/)).

XPath library functions are implemented with a special Rust binding system
based around Rust macros (defined in [`xee-xpath-macros`](xee-xpath-macros)),
which allows you to create Rust functions and register them into XPath.

XSLT support is very similar: XSLT XML is parsed, then turned into an XSLT AST
by [`xee-xslt-ast`](xee-xslt-ast). Any embedded XPath expressions are also
transformed into the XPath AST. The XSLT AST is then compiled into the IR by
[`xee-xslt-compiler`](xee-xslt-compiler)]. The IR and interpreter are shared with XPath.

## Project structure

### Crates

The Xee project is composed of many crates. Here is a quick overview:

- [`xee`](xee) - Swiss Army knife CLI tool for XML manipulation.

- [`xee-xpath`](xee-xpath) - Combines the underlying components to
  provide a high level API to support XPath queries in Rust.

- [`xee-testrunner`](xee-testrunner) - a testrunner that can run the
  QT3 conformance suite of XPath tests (in `vendor/xpath-tests`). It has also
  been generalized towards supporting running XSLT conformance tests, but that
  implementation is not complete yet.

- [`xee-xpath-lexer`](xee-xpath-lexer) - A lexer for XPath
  expressions.

- [`xee-xpath-ast`](xee-xpath-ast) - Defines an XPath AST. Turns
  `xee-xpath-lexer` output into an XPath AST.

- [`xee-xslt-ast`](xee-xslt-ast) - Parse XSLT documents into an AST. Uses
  `xee-xpath-ast` for the underlying XPath expressions.

- [`xee-xpath-compiler`](xee-xpath-compiler) - Compiles XPath AST provided by
  `xee-xpath-ast` to IR supported by `xee-ir`, which it then uses to create
  bytecode for `xee-interpreter`.

- [`xee-xslt-compiler`](xee-xslt-compiler) - A compiler of the XSLT AST
  (defined by `xee-xslt-ast`) into `xee-ir` IR, so that XSLT code can be run by
  the `xee-interpreter` engine.

- [`xee-ir`](xee-ir) - an intermediate language (in functional single
  assignment form) with logic to compile it down to Xee bytecode used by
  `xee-interpreter`.

- [`xee-interpreter`](xee-interpreter) - the core virtual machine interpreter
  that can execute XPath and XSLT. Also contains the XPath functions and
  operators implementation.

- [`xee-name`](xee-name) - support code for XML namespaces

- [`xee-xpath-macros`](xee-xpath-macros) - Macros used by `xee-interpreter` to
  help implement the XPath library functions. Provides a way to create Rust
  bindings for XPath, though is currently restricted to the functions and
  operator specification.

- [`xee-xpath-type`](xee-xpath-type) - The AST specifically for type
  expressions in the XPath AST. These are used separately by `xee-xpath-macros`
  for its Rust bindings infrastructure.

- [`xee-schema-type`](xee-schema-type) - support code defininig properties of
  core XML schema basic datatypes (`xs:*`).

- [`xee-xpath-load`](xee-xpath-load) - Infastructure to help defining loaders
  for XML data used by `xee-testrunner`.

Some affiliated projects exist as well maintained outside of this project:

- [`xot`](https://github.com/faassen/xot) - XML tree library implementation.
  Contains logic for traversal, manipulation, parsing and serialization.

- [`regexml`](https://github.com/Paligo/regexml) - XML Schema and XPath
  compatible regex engine.

- [`xee-php`](https://github.com/Paligo/xee-php) - PHP bindings for Xee.

### Other directories

- [`conformance`](conformance) - a manual tracking of various features of XPath
  and XSLT

- [`vendor`](vendor) - the QT3 test suite (`xpath-tests`) and `xslt-tests`
  vendored into this project for purposes of easy of access and stability.
