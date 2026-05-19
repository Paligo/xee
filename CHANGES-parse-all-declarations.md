# Feature: Parse All XSLT 3.0 Declarations & Graceful Degradation

## Summary

This change enables the XSLT parser and compiler to handle all 18 XSLT 3.0
top-level declaration types. Previously, only 5 declarations were parsed
(Accumulator, Template, Output, Mode, Function); the remaining 13 caused
immediate hard errors that prevented the entire stylesheet from compiling.

Now all declarations are parsed into the AST. Declarations that lack full
compiler support are gracefully skipped instead of aborting, allowing
stylesheets that *contain* unsupported declarations to still process their
templates.

## Test Results

| Metric | Before | After | Delta |
|--------|--------|-------|-------|
| Passed | 1,129 | 1,178 | **+49** |
| Failed | 604 | 738 | +134 |
| Error | 12,831 | 12,585 | **-246** |
| "Unsupported declaration" errors | ~9,003 | 37 | **-8,966** |

The +134 in "Failed" is actually progress — those tests now compile and run
but produce wrong results (due to skipped declarations like global variables),
rather than aborting outright. This makes them visible for future work.

## Changes

### 1. AST: `From<T> for Declaration` implementations (`xee-xslt-ast/src/ast_core.rs`)

Added `From<T> for Declaration` trait implementations for 13 types that were
missing them:

- `CharacterMap`, `DecimalFormat`, `GlobalContextItem`
- `Import`, `ImportSchema`, `Include`
- `Key`, `NamespaceAlias`
- `Param`, `PreserveSpace`, `StripSpace`
- `UsePackage`, `Variable`

These are required for the `DeclarationParser` trait to work (it requires both
`InstructionParser` and `Into<Declaration>`).

### 2. Parser: Hook up all declarations (`xee-xslt-ast/src/names.rs`)

Extended `DeclarationName::parse()` from 5 variants to 17 variants. Only
`UsePackage` remains unsupported (no `InstructionParser` implementation exists
for it yet; affects only 37 tests).

### 3. Compiler: Graceful skip (`xee-xslt-compiler/src/ast_ir.rs`)

Changed the `declaration()` method from returning `Err(Unsupported(...))` for
unknown declarations to returning `Ok(())` with explicit match arms:

- **Compiled:** `Template`, `Mode`, `Output` (as before)
- **Pre-processed:** `Import`, `Include` (handled during import resolution)
- **Skipped gracefully:** `Function`, `Variable`, `Param`, `Key`,
  `StripSpace`, `PreserveSpace`, `DecimalFormat`, `CharacterMap`,
  `NamespaceAlias`, `ImportSchema`, `UsePackage`, `GlobalContextItem`,
  `Accumulator`

### 4. Import cycle detection (`xee-xslt-compiler/src/ast_ir.rs`)

Added `HashSet<PathBuf>` tracking of visited stylesheet paths during
`xsl:import` and `xsl:include` processing. Previously, self-referential or
circular imports caused a stack overflow crash. Now they produce a clear error:
`"Circular import detected: '/path/to/file.xsl'"`.

## New Error Landscape

After these changes, the dominant error categories are:

| Error | Count | Cause |
|-------|-------|-------|
| XPST0008 Name not defined | 4,629 | Global variables/params skipped but referenced |
| Instruction not supported: SourceDocument | 1,677 | Test harness uses `<source-document>` |
| Failed parsing XSLT: Attribute | 1,352 | Various attribute parsing gaps |
| XPST0017 function not found | 911 | User-defined `xsl:function` not callable from XPath |
| No stylesheet found | 333 | Test runner can't locate stylesheet |

## Recommended Next Steps

1. **Global `xsl:variable` / `xsl:param` support** — Would resolve ~4,629
   XPST0008 errors. Requires architectural work to make global bindings
   available in template function scopes.

2. **`xsl:function` compilation** — Would resolve ~911 XPST0017 errors.
   Requires registering user-defined functions in the `StaticContext` so XPath
   expressions can resolve them.

3. **`xsl:source-document` instruction** — Would unblock ~1,677 tests. Many
   test stylesheets use this instruction to load input.

## Patch File

Saved as: `0001-parse-all-xslt-declarations-and-skip-gracefully.patch`

Apply with: `git apply 0001-parse-all-xslt-declarations-and-skip-gracefully.patch`
