# XSLT 3.0 conformance

Per element.

## xsl:accept

TODO: import subsystem

## xsl:accumulator

We don't go for streaming support.

## xsl:accumulator-rule

We don't go for streaming support.

## xsl:analyze-string

We have regexml now, so should be able to implement.

## xsl:apply-imports

Basic support with import precedence and non-tunnel `xsl:with-param`.

Not yet:

- Packages/expose/override
- Tunnel params

## xsl:apply-templates

Basic support:

- Named/unnamed/current modes
- Non-tunnel `xsl:with-param`
- Built-in templates via `xsl:mode` on-no-match

Not yet:

- Variables in patterns
- Certain axes

## xsl:assert

Basic support:

- Evaluates `test`; on failure raises `XTMM9001` or the supplied `error-code`.
- Uses `select` or the sequence constructor for the error message.

Not yet:

- Assertion disable/enable toggles outside `use-when` and the
  `enable_assertions` dependency.

## xsl:attribute

Cannot add after normal child.

Supports `type` (recorded in the type table; no schema validation).

Not yet:

- validation

## xsl:attribute-set

TODO

## xsl:break

TODO: xsl:iterate

## xsl:call-template

Basic support for named templates and non-tunnel `xsl:with-param`.

## xsl:catch

Supported as part of `xsl:try` (no `xsl:fallback`).

## xsl:character-map

TODO

## xsl:choose

Done

## xsl:comment

Done

## xsl:context-item

TODO

## xsl:copy

Supports `type` (recorded in the type table; no schema validation).

Not yet:

- copy-namespaces, inherit-namespaces, use-attribute-sets, validation

## xsl:copy-of

Supports `type` (recorded in the type table; no schema validation).

Not yet:

- copy-accumulators, copy-namespaces, validation

## xsl:decimal-format

TODO: awaiting xee-format

## xsl:document

Basic document node construction with sequence constructor content.
Supports `type` (recorded in the type table; no schema validation).

## xsl:element

Supports `type` (recorded in the type table; no schema validation).

Not yet:

- inherit-namespaces

- use-attribute-sets

- validation

## xsl:evaluate

TODO

## xsl:expose

TODO: import subsystem

## xsl:fallback

TODO

## xsl:for-each

TODO:

- xsl:sort support

## xsl:for-each-group

TODO

## xsl:fork

TODO

## xsl:function

Basic support for user-defined functions.

Not yet:

- Visibility/overriding/caching/streamability
- Default values for function parameters

## xsl:global-context-item

TODO

## xsl:if

Done

## xsl:import

Basic file-based import resolution with import precedence.

Not yet:

- Packages/expose/override

## xsl:import-schema

TODO: schema support

## xsl:include

Basic file-based include resolution.

## xsl:iterate

TODO

## xsl:key

TODO

## xsl:map

TODO

## xsl:map-entry

TODO

## xsl:matching-substring

TODO: regexml

## xsl:merge

TODO

## xsl:merge-action

TODO

## xsl:merge-key

TODO

## xsl:merge-source

TODO

## xsl:message

TODO

## xsl:mode

Supports `on-no-match` for built-in template behavior.

Not yet:

- Streamability and other mode attributes

## xsl:namespace

Not yet:

- validation that namespace cannot be added if a normal child has been added already.

## xsl:namespace-alias

TODO

## xsl:next-iteration

TODO: xsl:iterate

## xsl:next-match

Basic support with import precedence and non-tunnel `xsl:with-param`.

## xsl:non-matching-substring

Have regexml now.

## xsl:number

TODO: xee-format

## xsl:on-completion

TODO: xsl:iterate

## xsl:on-empty

TODO

## xsl:on-non-empty

TODO

## xsl:otherwise

Done

## xsl:output

TODO: output method subsystem

## xsl:output-character

TODO

## xsl:override

TODO: import subsystem

## xsl:package

TODO: import subsystem

## xsl:param

Supports global and template parameters (non-tunnel).

Not yet:

- Static params and visibility

## xsl:perform-sort

TODO

## xsl:preserve-space

TODO

## xsl:processing-instruction

Done

## xsl:result-document

TODO

## xsl:sequence

Done

## xsl:sort

TODO

## xsl:source-document

TODO

## xsl:strip-space

TODO

## xsl:stylesheet

Not yet: all of the attributes

## xsl:template

Including priority.

Named templates and mode selection are supported.

Not yet:

- match: variable support, certain axes

- as

- visibility

## xsl:text

Not yet:

- deprecated disable-output-escaping

## xsl:transform

See xsl:stylesheet

## xsl:try

Basic support for `xsl:try`/`xsl:catch`, including `rollback-output`.

Not yet:

- `xsl:fallback`

## xsl:use-package

TODO: import subsystem

## xsl:value-of

Done except:

- disable-output-escaping (backwards compatibility)

## xsl:variable

Basic support for global and local variables (non-static).

Not yet:

- static variables (compile-time)
- attributes: as, visibility

## xsl:when

Done

## xsl:where-populated

TODO

## xsl:with-param

Supported for apply-templates/apply-imports/next-match/call-template.

Not yet:

- Tunnel params
