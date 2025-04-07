# XSLT development plans

Here I will sketch out the plan for XSLT: where we are, what next steps
are, and how people could contribute.

## Current status

`xee-xslt-ast` parses the XSTL stylesheets into an AST. This AST is
very similar in structure to the underlying XML.

`xee-xslt-compiler` compiles this AST to IR (as defined by `xee-ir`).
All XPath expressions are compiled to an IR and are embedded in a larger IR.

Then `xee-ir` compiles this to bytecode which can be run by the interpreter.

`xee-xslt-compiler/tests/test_xslt.rs` has hand-written tests for particular
XSLT features.

XSLT support is decidedly partial now; various parts of XSLT are implemented
but a lot is missing.

## Where we want to be

We want XSLT to be implemented fully.

We want a `xee-xslt` with a public API that lets developers execute XSLT from
Rust.

We want that integrated in the `xee` CLI.

## How to get there

### xee-xslt-ast

This is the easiest to step into right now. `xee-xslt-ast` is fairly complete
already but there are some details to be completed.

So we want to harden the `xee-xslt-ast` so we know it can truly load all valid
XSLT (and reject invalid XSLT). Right now this is done via snapshot tests using
`insta` in `xee-xslt-ast/tests/snapshot_tests.rs`. 
We may want to integrate a way to snapshot test XSLT files (manually
verifying whether the AST is plausible and doesn't error when we don't expect
it to be).

Whenever we have unexpected behavior, we should extend the XSLT parser to
handle this.

This won't make any more XSLT run but it ensures what we load is correct.

### xee-testrunner

We want `xee-testrunner` to be able to execute the XSLT test suite in
`vendor/xslt`. Then we can slowly build up test coverage. Martijn has done
preparatory work and is working towards the ability to run our first XSLT
conformance tests.

### xee-xslt-compiler

We want to extend `xee-xslt-compiler` so it can compile more XSLT constructs to
the IR. The compilation code is in `src/ast_ir.rs`. We can extend the tests in
`test/test_xslt.rs` but the test runner once it works can also help drive this.

Right now we don't have snapshot tests to verify that particular AST gets
transformed into particular IR, but it may be useful to add this.

### Extending the IR and interpreter

In some cases the IR or underlying interpreter are likely insufficient to
support an XSLT feature and need extension. Serialization options already have
a lot of preparation in Xot but need integration, for instance.

### Error messages

Ideally we want great error messages. `xee-xslt-ast` has an error system
already but we want to make them more readable and possibly extend it. XPath
runtime errors leave a lot to be desired and we need a plan to make them
better.
