# xee-xpath-compiler
[![Crates.io](https://img.shields.io/crates/v/xee-xpath-compiler.svg)](https://crates.io/crates/xee-xpath-compiler)
[![Documentation](https://docs.rs/xee-xpath-compiler/badge.svg)](https://docs.rs/xee-xpath-compiler)


A compiler to compile XPath text to Xee IR (intermediate representation). This
makes use of [`xee-xpath-ast`](https://crates.io/crates/xee-xpath-ast).
[`xee-ir`](https://crates.io/crates/xee-ir) defines the intermediate
representation and allows you to compile it to bytecode as defined by
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