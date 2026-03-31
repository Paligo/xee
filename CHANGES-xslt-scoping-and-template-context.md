# Feature: XSLT Variable Scoping, Named-Template Focus, and Sequence Constructor Fixes

## Summary

This change is a follow-up to the earlier parameter-runtime work. Its focus is
not new error codes, but correctness of variable and parameter binding across
XSLT and shared XPath lowering.

The main areas improved here are:

1. lexical scoping for XSLT local variables and params
2. context/focus plumbing for named templates
3. XPath declaration scoping for `let`, `for`, quantified expressions, and
   inline function parameters
4. sequence-constructor lowering so `xsl:variable` is handled correctly even
   when it does not appear as the first item in a constructor

This pass fixes a real cluster of declaration failures, especially those that
previously reported internal compiler errors around missing variables or
mis-bound shadowed variables.

## What Was Implemented

### 1. Lexical variable scopes in the shared IR variable table

Updated `xee-ir/src/variables.rs` so variable bindings are no longer stored in
one flat map for the entire conversion pass.

The `Variables` helper now maintains a stack of lexical scopes and exposes
separate operations for:

- looking up an existing variable binding
- declaring a fresh local binding in the current scope
- entering and exiting a lexical scope

This fixes two correctness problems:

- local variables and params can now shadow outer bindings correctly
- a variable is no longer visible within its own initializer unless an outer
  binding of the same name exists

That matters for both XSLT local variable declarations and the shared XPath IR
converter used to compile expressions inside XSLT instructions.

### 2. Template-local scopes are isolated between separately compiled templates

Updated `xee-xslt-compiler/src/ast_ir.rs` so each template compilation pushes a
fresh variable scope before registering its params and lowering its body.

Before this change, template-local parameter bindings could leak across
templates because the compiler reused the same variable table while compiling
the whole stylesheet.

The result was a class of bogus internal failures where one template body could
end up referencing a runtime name that had only been declared while compiling a
different template.

This was visible in cases such as `param-0107`, where the unresolved variable
coming out of lowering was not from the current template at all.

### 3. Named templates now receive the current focus triple

Updated the XSLT AST lowering and IR call path so named templates receive the
current focus, matching what matched templates already received.

Concretely:

- `xee-ir/src/ir.rs` now records optional call-site context on
  `ir::CallTemplate`
- `xee-xslt-compiler/src/ast_ir.rs` captures the current context when lowering
  `xsl:call-template`
- named template function definitions now include hidden leading params for:
  - current item
  - current position
  - current size
- `xee-ir/src/declaration_compiler.rs` now registers only the explicit template
  params for name validation, skipping those hidden focus params
- `xee-ir/src/function_compiler.rs` passes either the current focus triple or
  explicit absent markers when compiling a named-template call

This fixes named-template bodies and defaults that depend on the current node,
for example template params with defaults such as `select="@id"`.

### 4. Matched and named template param defaults now compile with the right scope

The earlier apply-templates work already moved matched-template defaults under a
live context. This pass tightens the surrounding lexical scope handling so both
matched and named templates compile defaults while the correct template-local
bindings are visible and isolated.

That improves cases where one param default references another param, and cases
where a default references the context item.

### 5. Sequence constructors now handle `xsl:variable` anywhere, not just first

Updated `xee-xslt-compiler/src/ast_ir.rs` so sequence-constructor lowering no
longer assumes that only the first item in a constructor might be a variable
declaration.

Previously:

- `xsl:variable` was recognized only if it appeared at the head of the current
  constructor slice
- a later `xsl:variable` would fall through into normal instruction lowering
- that triggered the internal error path:
  `Internal bug: variable node should have been processed already`

The new recursive lowering handles variable declarations anywhere in the
constructor while still preserving `let` nesting semantics.

This directly fixed `param-0107`.

### 6. XPath declaration sites now use real lexical declaration scopes

Updated `xee-xpath-compiler/src/ast_ir.rs` so XPath declaration forms no longer
use the old “lookup or create” variable registration behavior.

Adjusted sites include:

- `let` expressions
- `for` expressions
- quantified expressions
- inline function parameters

These declaration sites now push a fresh scope, declare their binding names in
that scope, lower the body, and then pop the scope.

This is necessary because XSLT param and variable expressions can contain XPath
shadowing forms such as:

```xpath
($y, (for $y in ('x', 'y', 'z') return $y))
```

Without lexical declaration scopes, the inner `$y` could be compiled against
the wrong binding or reuse an outer runtime name.

## Files Changed

### IR and Compiler Infrastructure

- `xee-ir/src/variables.rs`
- `xee-ir/src/ir.rs`
- `xee-ir/src/declaration_compiler.rs`
- `xee-ir/src/function_compiler.rs`

### XSLT Lowering

- `xee-xslt-compiler/src/ast_ir.rs`

### XPath Lowering

- `xee-xpath-compiler/src/ast_ir.rs`

## Verification

### Focused Rust crate tests

Verified with:

```bash
cargo test -q -p xee-ir -p xee-interpreter -p xee-xslt-compiler
```

Result: all tests passed.

### Targeted conformance cases rerun during this pass

Explicitly rerun and confirmed fixed:

- `param-0107`

Explicitly rerun and confirmed improved but still not fully correct:

- `variable-3301`
  - no longer fails with an internal compiler error
  - now reaches result comparison and fails with an XML mismatch

### Aggregate conformance snapshot after this change

`decl/param`:

- Passed: 13
- Failed: 0
- Error: 11
- WrongE: 7

`decl/variable`:

- Passed: 58
- Failed: 10
- Error: 32
- WrongE: 8

Compared to the previous checkpoint, this is a net improvement in both suites,
especially by removing part of the internal-variable-resolution failure class.

## What This Does Not Yet Solve

Several known areas remain open:

1. tunnel-parameter semantics are still incomplete
2. some remaining `decl/variable` failures now show behavioral mismatches rather
   than compiler crashes
3. unsupported instruction families such as `next-match`, `apply-imports`, and
   `for-each-group` still account for some remaining declaration-suite failures

One useful sign of progress is that a number of reducers that previously failed
with internal compiler errors now fail later and more specifically, which means
the binding model is materially closer to correct.

## Commit Readiness

This is a reasonable checkpoint commit.

Why it is commit-worthy:

- it fixes real correctness bugs in shared variable scoping
- it removes a class of internal compiler errors rather than only changing test
  outcomes superficially
- focused Rust regression tests are green
- declaration-suite pass counts improved in both `param` and `variable`
- it is logically cohesive with the current XSLT declaration work

Why it is not a final declaration-semantics commit:

- tunnel parameters are still incomplete
- some template/application behavior is still wrong even though the compiler no
  longer crashes
- several remaining failures are due to still-unsupported XSLT instruction
  families

Suggested commit framing:

- `Fix XSLT variable scoping and named-template context`
- or `Improve XSLT template param scoping and context binding`

## Next Step

The next highest-value target is likely one of:

1. the tunnel-parameter cluster around tests such as `variable-0203`
2. the remaining behavioral mismatch in `variable-3301`
3. unsupported instruction families that are now standing out more clearly
   after the scoping fixes