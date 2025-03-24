# xee-xslt-ast

Parser and AST support for XSLT 3.0. This defines the AST representing XSLT as
Rust objects. It is used by
[`xee-xslt-compiler`](https://crates.io/crates/xee-xslt-compiler) to compile
XSLT to Xee IR as defined by [`xee-ir`](https://crates.io/crates/xee-ir) which
can then be compiled down into bytecode and executed using
[`xee-interpreter`](https://crates.io/crates/xee-interpreter).

This is a low-level crate of the [Xee project](https://github.com/Paligo/xee).
For the API entry point see
[`xee-xpath`](https://docs.rs/xee-xpath/latest/xee_xpath/). For the `xee`
commandline tool, download a
[release](https://github.com/Paligo/xee/releases/).

## More Xee

[Xee homepage](https://github.com/Paligo/xee)

## Credits

This project was made possible by the generous support of
[Paligo](https://paligo.net/).