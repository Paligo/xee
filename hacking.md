# Hacking guide

In this guide we give an idea of various development tasks and how to approach
them. To gain a basic understanding, first read the README and get an overview
of the architecture.

We don't want to add functionality without testing it. This may require you
having to write new tests yourself, but in the case of XPath we can rely on a
conformance test suite of more than 20,000 tests.

## XPath

### Xpath functions

The [XPath and XQuery Functions and
Operators](https://www.w3.org/TR/xpath-functions-31/) specification describes
what various functions should exist and what they should do.

Now it could be that there's a flaw in an existing implementation, or you
want to add a new one.

In `xee-interpreter/src/library` you find the implementation of the XPath
standard library. When implementing a new function it's useful to read a few
existing ones to learn more about how they handle things.

You see it uses a macro-based approach to define functions:

```
#[xpath_fn("fn:node-name($arg as node()?) as xs:QName?", context_first)]
```

This defines the function with the type signature as defined in the
specification. The second argument, in this case `context_first` is optional,
but lets you define special behaviors. [`fn:node-name()`](https://www.w3.org/TR/xpath-functions-31/#func-node-name) has actually two possible signatures:

```
fn:node-name() as xs:QName?
fn:node-name($arg as node()?) as xs:QName?
```

The version without the first argument is defined by passing the context item
as the first argument automatically. `context_first` automatically generates
this version of the function as well, so you don't have to do this manually.

The macro automatically translates XPath type arguments to Rust types. You can
find examples in the existing functions to see what you can do. For the return
value there is no checking, so you have to ensure that the result value matches
the one in the type. Since the macro does an `.into()` to the declared return
type in the end, it may well work.

Any function may return an `error::Result` in case the XPath function needs to
return an error. 

Sometimes you need information from the interpreter to implement the function -
particularly the context (dynamic context, static context) and the `xot` object
from the interpreter. Any function can take special optional arguments
`context` and `interpreter` as the first two arguments. In this case the system
automatically injects these objects into your function. 

One you've created a function it needs to be registered at the bottom in
`static_function_descriptions` using the `wrap_xpath_fn!` macro.

In case you're creating a new library module, you need to hook up your new
`static_function_descriptions` in `src/library/mod.rs` as well.

You of course want to run tests to verify whether your changes have worked.

### Running the XPath tests

`cargo test` in the project runs all the Rust-based tests. But we have many
more tests in the form of a large conformance test suite.

You execute this by going to the `xee-testrunner` runner and running:

```
cargo run --release  -- check ../vendor/xpath-tests/
```

We use `--release` as we want the tests to run as quickly as possible. The
`check` command indicates that we only want to do regression tests - rerun the
tests we already *know* should pass as they did in the past.

This gives a result in the end that reads like this:

```
Total: 31812 Supported: 21859 Passed: 19987 Failed: 0 Error: 0 WrongE: 0
Filtered: 1872 Unsupported: 9953
```

`Total` is the total amount of tests in the suite. This includes features that
we don't support, most prominently XQuery, so `Supported` is the total amount
of tests that are relevant to use. `Passed` indicates how many of the tests
that behave as expected, `Failed`, the tests that failed (wrong answers),
`Error` the tests that had an unexpected error, `WrongE` those tests that
expect an error but the wrong error is returned. 

`Filtered` is those tests we want to support but do not work yet - we know they
fail in advance so they're filtered out by `check`.

You can also run all supported tests, including those known to fail:

```
cargo run --release  -- all ../vendor/xpath-tests/
```


If you have implemented new functionality or done a bugfix, you want to find
out which tests now pass which didn't before and update the filter to include
those as well. You can do this with the `update` command:

```
cargo run --release  -- update ../vendor/xpath-tests/
```

After running `update` you should always run `check` again to ensure there
are no regressions - `Failed`, `Error` and `WrongE` should remain at 0.

### Zooming in on tests

You can run `all` against a whole test xml file. To rerun just the `node-name`
tests.

```
cargo run --release  -- -v all ../vendor/xpath-tests/fn/node-name.xml
```

Thanks to the `-v` option you can see the test names and you can also see more
information about test passing and failure.

You can also filter tests further by using (part of) its name in the XML file

```
cargo run --release  -- -v all ../vendor/xpath-tests/fn/node-name.xml fn-node-name-1
```

This only runs those tests that have the `fn-node-name-1` string in them.

### Clearing the test filters

In some rare cases a fix actually breaks other tests because while it was
passing before, this was only a coincidence due to it erroring out due to
missing functionality. If you are REALLY sure that you want to accept this
breaking tests, you can *clear the filters*.

You do this by removing `vendor/xpath-tests/filters` (or moving it to another file)
and then rerunning:

```
cargo run --release  -- initialize ../vendor/xpath-tests/
```

This regenerates the `filters` file from scratch. This means that are newly
failing are added to it, *decreasing* the total amount of tests that pass
successfully. When you do this it makes sense to do a diff with the previous
version of `filters` to see whether you've made any mistakes and caused too
many tests to fail.

### Writing a Rust test

You may have a thorny problem and want to write a Rust-based test manually
instead to make debugging more easy. Such tests currently exist in
`xee-xpath/tests`.  You can take full control in these tests using the
`xee-xpath` API, including building up a context dynamically and providing it
with the exact context you want it to have.

## XSLT

### Testing XSLT

Manual Rust-based XSLT tests can be written in `xee-xslt-compiler/tests`.

To run the XSLT conformance tests you can now do:

```
cargo run --release  -- check ../vendor/xslt-tests/
```

You can improve tests just as described for XPath.

Note that some features of the XSLT test runner such as passing in parameters
are not yet working correctly; so if a test fails you may also want to suspect
the test runner setup, not just the implementation. Like for the XPath
conformance tests we intend to improve support for the testrunner
incrementally.

### Adding XSLT functionality

The XSLT AST is pretty complete, and underlying IR and bytecode interpreter
supports a lot of XSLT functionality already. Much of the effort of adding XSLT
functionality is focused on translating the XSLT AST into the IR format. This
is done by `xee-xslt-compiler/src/test_xslt.rs`. 
