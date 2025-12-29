# Conformance

Xee contains an extensive implementation of XPath 3.1, and a partial
implementation of XSLT 3.0. Both use the same compiler/interpreter framework.

## XPath conformance

Xee does not support XML schemas at present. Only basic atomic types are
supported. Optional static typing is not supported, but dynamic type checking
does take place.

XPath 1.0 mode is not supported.

### XPath functions

Most of the `fn` library has been implemented. `math` functions are supported.
`map` and `array` are both fully supported.

In general, gaps still exist in parsing and formatting, unparsed text, document
collections, and JSON support.

See [fn.md](fn.md) for details.

### Tests

## XSLT conformance

XSLT is parsed into a complete AST and a large subset of XSLT works, but
there are gaps all over the place.

`xsl:template`, `xsl:value-of`, `xsl:variable`, `xsl:if` `xsl:choose`,
`xsl:when`, `xsl:otherwise`, `xsl:for-each`, `xsl:copy`, `xsl:copy-of`,
`xsl:sequence`, `xsl:apply-templates`, `xsl:apply-imports`, `xsl:next-match`,
`xsl:call-template`, `xsl:try`, `xsl:catch`, `xsl:text`, `xsl:attribute`,
`xsl:namespace`, `xsl:comment`, `xsl:processing-instruction` all have their
core behavior implemented.

See [xslt.md](xslt.md) for details.

### Tests

The XSLT test suite can now be run with the test runner, but coverage is still
partial and many tests are filtered. The test runner continues to improve as
more XSLT functionality lands.
