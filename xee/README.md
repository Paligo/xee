# xee

Swiss Army knife tool for XML.

The `xee` (try `-h`) allows you to do stuff with XML on the command line.

Features include:

- formatting XML in various ways, including in indented form
- evaluate an XPath expression against an XML document
- a REPL for evaluating XPath expressions.

This implements XPath 3.1.

## Usage

### Execute an XPath expression

Execute an XPath expression `/doc/p` against a file `foo.xml`, result to stdout:

```
xee xpath /doc/p foo.xml
```

### Interactive shell for XPath

Interactive shell (REPL) to issue multiple xpath expressions against a document:

```
xee repl
```

### Pretty-print an XML file

Pretty-print `foo.xml`, result to stdout:

```
xee indent foo.xml
```

## More Xee

This is built using [`xee-xpath`](https://docs.rs/xee-xpath/latest/xee_xpath/),
a high level API to issue XPath 3.1 expressions in Rust.

[Xee homepage](https://github.com/Paligo/xee)

## Credits

This project was made possible by the generous support of
[Paligo](https://paligo.net/).

