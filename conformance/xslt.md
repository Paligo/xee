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

TODO: import subsystem

## xsl:apply-templates

Not yet:

- Mode support

- Variables in patterns

- Rooted patterns

- Certain axes

- Fallback templates

## xsl:assert

Basic support:

- Evaluates `test`; on failure raises `XTMM9001` or the supplied `error-code`.
- Uses `select` or the sequence constructor for the error message.

Not yet:

- Assertion disable/enable toggles outside `use-when`.

## xsl:attribute

Cannot add after normal child.

Not yet:

- type

- validation

## xsl:attribute-set

TODO

## xsl:break

TODO: xsl:iterate

## xsl:call-template

TODO: function subsystem

## xsl:catch

TODO

## xsl:character-map

TODO

## xsl:choose

Done

## xsl:comment

Done

## xsl:context-item

TODO

## xsl:copy

Not yet:

- copy-namespaces, inherit-namespaces, use-attribute-set, type, validation

## xsl:copy-of

Not yet:

- copy-accumulators, copy-namespaces, type, validation

## xsl:decimal-format

TOD: awaiting xee-format

## xsl:document

TODO: nodes

## xsl:element

Not yet:

- inherit-namespaces

- use-attribute sets

- type

- validation

## xsl:evaluate

TODO

## xsl:expose

TODO: import subsystem

## xsl:fallback

TODO

## xsl:for-each

Todo:

- xsl:sort support

## xsl:for-each-group

TODO

## xsl:fork

TODO

## xsl:function

TODO: function subsystem

## xsl:global-context-item

TODO

## xsl:if

Done

## xsl:import

TODO: import subsystem

## xsl:import-schema

TODO: schema support

## xsl:include

TODO: imoprt subsystem

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

TODO

## xsl:namespace

Not yet:

- validation that namespace cannot be added if a normal child has been added already.

## xsl:namespace-alias

TODO

## xsl:next-iteration

TODO: xsl:iterate

## xsl:next-match

TODO: template rule subsystem, import system

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

TODO

## xsl:output

TODO: output method subsystem

## xsl:output-character

TODO

## xsl:override

TODO: import subsystem

## xsl:package

TODO: import subsystem

## xsl:param

TODO: function subsystem

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

Not yet: all of the attibutes

## xsl:template

Including priority.

Not yet:

- match: variable support, rooted paths, certain axes

- name

- mode

- as

- visibility

## xsl:text

Not yet:

- depecrated disable-output-escaping

## xsl:transform

See xsl:stylesheet

## xsl:try

TODO

## xsl:use-package

TODO: import subsystem

## xsl:value-of

Done except:

- disable-output-escaping (backwards compatibility)

## xsl:variable

Not yet:

- compile-time variables used as global variables

- global variables

- attributes: as, visbiility

## xsl:when

Done

## xsl:where-populated

TODO

## xsl:with-param

Todo: function subsystem
