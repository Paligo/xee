# Feature: XSLT Param Validation, Required Param Runtime Errors, and Test Runner Wiring

## Summary

This change completes a focused pass on XSLT parameter handling around three
areas that were previously missing or only partially wired:

1. Static validation for invalid `xsl:param` declarations
2. Dynamic errors for missing required stylesheet/template parameters
3. Test runner support needed to exercise these semantics correctly

The implementation does not yet solve parameter propagation for
`xsl:apply-templates`. That remains the next major gap. But it does make the
named-template and global-parameter behavior materially more correct and fixes
several targeted conformance cases.

## What Was Implemented

### 1. Static validation for invalid required params

Added validation in `xee-xslt-compiler/src/ast_ir.rs` so that an `xsl:param`
with `required="yes"` cannot also provide a `select` attribute or child
sequence constructor.

This now raises `XTSE0010` during compilation instead of silently compiling the
stylesheet.

Applied to:

- global `xsl:param`
- template-local `xsl:param`

### 2. IR now records whether a parameter is required

Extended `xee-ir/src/ir.rs` so `ir::Param` carries a `required: bool` field.

This is important because requiredness must survive lowering from XSLT AST into
IR and then into bytecode generation, especially for named templates.

### 3. Required template params now fail at runtime

Updated `xee-ir/src/function_compiler.rs` to emit explicit runtime checks for
required parameters that do not have defaults.

Implementation details:

- existing default-parameter handling already used `VarIsAbsent`
- required params now also use `VarIsAbsent`
- if the argument is absent, the compiler emits a dedicated bytecode raise path
- the interpreter maps that to `XTDE0700`

This keeps required-param behavior localized and avoids overloading unrelated
error paths.

### 4. Required global stylesheet params now fail at runtime

Updated `xee-interpreter/src/interpreter/interpret.rs` so global stylesheet
params marked `required` now raise `XTDE0050` if no externally supplied value
exists in the dynamic context.

Before this change, missing required stylesheet params simply fell through and
executed as if no error existed.

### 5. Named template parameter matching is stricter

Updated named-template lookup/matching in `xee-ir/src/declaration_compiler.rs`
and `xee-ir/src/function_compiler.rs`.

Behavior changes:

- named templates are keyed by stable local-name strings instead of debug
  formatting
- `xsl:with-param` names are checked against the declared params of the target
  named template
- undeclared non-tunnel params now raise `XTSE0680`

This fixes cases where `call-template` supplied a parameter the callee never
declared.

### 6. Added dedicated bytecode support for raising required-param errors

Added `RaiseError(RaisedError)` to
`xee-interpreter/src/interpreter/instruction.rs` with a current specialized
variant for `XTDE0700`.

This keeps the runtime implementation simple:

- compiler emits a specific instruction
- interpreter decodes it and returns the correct XSLT error

The goal was to avoid a larger general-purpose exception mechanism for this
targeted fix.

### 7. Test runner now passes stylesheet variables into the dynamic context

Updated `xee-testrunner/src/testcase/xslt.rs` so XSLT tests actually load the
environment variables and pass them into the program's dynamic context.

Without this, required global-param tests could not be evaluated correctly,
because externally supplied stylesheet parameters were ignored by the runner.

### 8. Test runner now honors `initial-template`

Updated `xee-testrunner/src/testcase/xslt.rs` and
`xee-interpreter/src/interpreter/runnable.rs` so XSLT conformance tests can run
an initial named template instead of always starting from the normal main
template path.

Implementation details:

- parse `<initial-template name="..."/>` from the test definition
- look up the named template in runtime declarations
- invoke it directly through the interpreter
- supply explicit absent placeholders for unsupplied template arguments so
  required-param checks run in the callee rather than failing early on arity

This was necessary for `param-0116` to test the intended behavior.

### 9. XSLT compile errors are now surfaced as compilation errors in the runner

Updated `xee-testrunner/src/testcase/xslt.rs` so stylesheet compilation errors
are handled like XPath compilation errors:

- if the test expects an error, compare against the error code
- otherwise report a compilation error

Previously these cases were often classified as generic environment errors,
which made static-error conformance checks less meaningful.

## Files Changed

### Compiler and IR

- `xee-xslt-compiler/src/ast_ir.rs`
- `xee-ir/src/ir.rs`
- `xee-ir/src/declaration_compiler.rs`
- `xee-ir/src/function_compiler.rs`
- `xee-xpath-compiler/src/ast_ir.rs`
- `xee-ir/tests/test_xml_ir.rs`

### Interpreter

- `xee-interpreter/src/error.rs`
- `xee-interpreter/src/function/inline_function.rs`
- `xee-interpreter/src/declaration/decl.rs`
- `xee-interpreter/src/interpreter/instruction.rs`
- `xee-interpreter/src/interpreter/interpret.rs`
- `xee-interpreter/src/interpreter/runnable.rs`

### Test Runner

- `xee-testrunner/src/testcase/xslt.rs`

## Error Codes Added or Wired In

The following XSLT error codes are now present and used by this change:

- `XTSE0010` for invalid `required="yes"` declarations with a default/select
- `XTDE0050` for missing required global stylesheet params
- `XTSE0680` for undeclared `xsl:with-param` on `xsl:call-template`
- `XTDE0700` for missing required template params at runtime

## Verification

### Focused Rust crate tests

Verified with:

```bash
cargo test -q -p xee-interpreter -p xee-ir -p xee-xslt-compiler -p xee-testrunner
```

Result: all tests passed.

### Targeted conformance cases fixed

These cases were explicitly rerun and now pass:

- `param-0111`
- `param-0112`
- `param-0116`
- `variable-2202`

### Aggregate conformance snapshot after this change

`decl/param`:

- Passed: 11
- Failed: 0
- Error: 13
- WrongE: 7

`decl/variable`:

- Passed: 45
- Failed: 4
- Error: 49
- WrongE: 10

## What This Does Not Yet Solve

The main remaining semantic gap is still `xsl:apply-templates` parameter
transport and matched-template params.

Known example still failing:

- `variable-1201` produces `<out>ABC_ABC</out>` instead of `<out>ABC_25</out>`

That indicates:

1. `xsl:apply-templates` is still dropping `xsl:with-param`
2. matched templates still are not consuming local `xsl:param` as callable
   parameters

## Commit Readiness

This is a reasonable checkpoint commit.

Why it is commit-worthy:

- it is internally coherent
- crate-level regression tests are green
- several targeted conformance cases are now fixed
- the runner changes are necessary for the parameter work to be testable at all
- the remaining failure area is clearly isolated to `apply-templates` param
  propagation, not to this required-param work

Why it is not a "finished param system" commit:

- `apply-templates` parameter propagation is still missing
- aggregate declaration suites still contain substantial `Error` and `WrongE`
  counts

Suggested commit framing:

- "Add XSLT required param validation and runner support"
- or "Implement XSLT param validation and initial-template runner support"

## Next Step

The next implementation target should be end-to-end support for
`xsl:apply-templates` parameters:

1. extend IR `ApplyTemplates` to carry params
2. lower `ApplyTemplatesContent::WithParam`
3. pass params through runtime template dispatch
4. compile matched templates with consumable template params