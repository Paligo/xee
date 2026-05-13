# Extension functions — design specification

A specification for exposing Xee's existing `#[xpath_fn]` machinery to
external crates, so that downstream consumers can register their own
XPath function libraries (`xfi:*`, `xff:*`, application-specific
functions) without forking `xee-interpreter`.

This is the design doc that the upstream PR(s) will draw from. Nothing
here is yet committed in code.

## 1. Motivation

The upstream README states:

> The Rust binding system for XPath can only be used to implement
> standard library functions — support for extension functions needs to
> be created.

Concretely, this blocks Xee's use as the XPath engine in XBRL formula
evaluators. The XBRL Functions Instance library (`xfi:*`) defines ~80
functions over XBRL semantics — period accessors, context comparison,
dimension witnesses, relationship lookups — none of which belong in a
generic XPath engine but all of which appear throughout real XBRL
taxonomies and across the [Formula 1.0 conformance suite][formula-1.0]
(30+ distinct `xfi:*` functions referenced). Without external
registration, Xee compiles every one of these to `XPST0017`
("undefined function") and cannot evaluate the bulk of conformance-
suite assertions.

[formula-1.0]: https://specifications.xbrl.org/work-product-index-formula-formula-1.0.html

The same gap shapes any non-XBRL Xee adoption that needs a custom
library: XSpec-style assertion helpers, application-specific date/locale
functions, schematron extensions, and so on.

The *good news* is that the registration machinery already exists
internally — `#[xpath_fn]` plus `wrap_xpath_fn!` already covers
~125 standard-library functions across `xee-interpreter/src/library/`.
The work is exposing that machinery to crate consumers cleanly,
without breaking existing internal use and without committing to a
public API surface that boxes in future evolution.

## 2. Goals and non-goals

### 2.1 Goals

1. Allow external crates to register XPath functions under any
   namespace, using a Rust API that mirrors the existing
   `#[xpath_fn]` developer experience for internal functions.
2. Permit registered functions to receive a `&DynamicContext` and
   `&mut Interpreter` (as internal functions do) **and** application-
   specific state (an XBRL discoverable taxonomy set (DTS) handle,
   a database connection, etc.) without forcing thread-local globals
   on the consumer.
3. Keep registration zero-cost for consumers that don't need it — Xee's
   built-in library should continue to work unchanged, with no perf
   regression on the common path.
4. Lay out an upstream-mergeable path: small, reviewable PRs; minimal
   churn to internal crate boundaries; explicit semver implications.
5. Document the API surface as **stable enough to commit to** for
   `xee-xpath` 0.x but not yet 1.0 — i.e. SemVer-minor breakage
   allowed in the 0.x line, with deprecation aids where reasonable.

### 2.2 Non-goals

- **Pluggable XML data model.** Allowing consumers to feed Xee a
  non-Xot tree (or a typed-data adapter) is a separate question.
  This document assumes consumers continue to ingest XML via
  `Documents::add_string`.
- **Custom function declarations in XPath itself** (`declare function`,
  XQuery-style). Out of scope.
- **Higher-order extension functions** (functions returning functions,
  partial application of extension functions). Out of scope for v1; see
  §10.4.
- **Async / non-Send extension functions.** Xee is single-threaded
  today; user functions inherit that constraint.
- **Hot reloading or unregistering** functions after a static context
  is built.
- **Function shadowing rules** beyond "later registrations override
  earlier ones with the same `(name, arity)`". No diamond resolution,
  no namespace import precedence beyond what XPath already specifies.

## 3. Current state

References below are to file paths in this repository.

### 3.1 The registration pipeline today

```
#[xpath_fn("ns:name(...) as ...")]      (xee-xpath-macros)
fn my_fn(...) -> ... { ... }
        │
        ▼
generates a module `my_fn` with WRAPPER / SIGNATURE / KIND constants
        │
        ▼
wrap_xpath_fn!(my_fn)                    (xee-interpreter)
        │
        ▼
StaticFunctionDescription { name, signature, kind, func }
        │
        ▼ (collected in library/mod.rs::static_function_descriptions)
        ▼
StaticFunctions::new()                   (called once via LazyLock)
        │
        ▼
&'static StaticFunctions                 (referenced by every
                                          StaticContext)
```

Key types and locations:

- `StaticFunctionType` — the bare-fn-pointer signature shared by every
  XPath function: `xee-interpreter/src/function/static_function.rs:55-59`.
- `StaticFunctionDescription` — descriptor produced by
  `wrap_xpath_fn!`: `static_function.rs:61-66`.
- `wrap_xpath_fn!` macro — gathers the macro-generated constants into
  a `StaticFunctionDescription`: `static_function.rs:70-82`.
- `StaticFunctions` registry — owns the `Vec<StaticFunction>` indexed
  by `StaticFunctionId`: `static_function.rs:269-320`.
- Singleton instantiation:
  `xee-interpreter/src/context/static_context.rs:17-18`
  (`LazyLock<StaticFunctions>`) and `:30` (`&'static StaticFunctions`).
- Compiler lookup by qname during AST→IR lowering:
  `xee-xpath-compiler/src/ast_ir.rs:601-628`
  (`static_context.function_id_by_name(&qname, arity)`).
- Bytecode emission of the ID:
  `xee-ir/src/function_compiler.rs:118-120, :382-421`
  (`ir::Const::StaticFunctionReference(id, _)` → `Instruction::StaticClosure(id.as_u16())`).
- Builder surface today:
  `xee-interpreter/src/context/static_context_builder.rs` —
  namespaces, variable names, default element/function NS, static base
  URI. **No function-registration entry point.**

### 3.2 The four structural constraints

**Constraint A — singleton registry.** `StaticContext` references
`StaticFunctions` by `&'static`. Every `StaticContext::new` call gets
the same pointer to the same library. There is no place today to put
a per-context or per-builder function table.

**Constraint B — bytecode interns the function ID.** `StaticFunctionId`
is a `usize` index into `StaticFunctions::by_index`. The compiler
writes it into bytecode via `Instruction::StaticClosure`. So the ID
must be stable *for as long as a compiled `Program` is live*. Two
`StaticContext`s with different tables cannot share bytecode.

**Constraint C — no user-data slot on `DynamicContext`.** The dynamic
context exposes `program`, `context_item`, `documents`, `variables`,
`current_datetime`, collections, env vars (see
`xee-interpreter/src/context/dynamic_context.rs:20-44`). Functions
that need application state (DTS, database, filesystem access beyond
`fn:doc`) have nowhere to read from.

**Constraint D — signature parse needs namespaces.** The `wrap_xpath_fn!`
macro parses the signature string at registration time using
`xee_name::DEFAULT_NAMESPACES` (`static_function.rs:74, :94-95`).
`DEFAULT_NAMESPACES` knows `fn:`, `xs:`, `math:`, etc. — but not
`xfi:`, `xff:`, or arbitrary user prefixes. The signature
`xfi:c-equal(...)` will fail to parse unless we extend the namespace
binding visible at registration time.

These four constraints are what the design has to address.

## 4. Use cases driving the API

The first three drive the v1 design; the fourth shapes which doors we
must not slam shut.

### 4.1 Pure-function registration (no app state)

A consumer registers ~10–15 stateless `xfi:*` helpers (period
accessors, instance comparisons that only need the context node).
The function body reads only the XPath `DynamicContext` (context
item, current node) and the `xot::Xot` tree available via
`context.documents()`.

```rust
#[xpath_fn("xfi:is-instant-period() as xs:boolean", context_first)]
fn is_instant_period(node: &Node) -> bool {
    // walk to enclosing xbrli:context and inspect xbrli:period
    ...
}
```

Required: no user-data plumbing; only the macro and `add_function` on
`StaticContextBuilder`. This is the *minimum useful* surface.

### 4.2 Stateful function registration (read-only app handle)

A consumer registers ~5–10 `xfi:*` functions that need read-only
access to the DTS (concept lookups, dimension membership tests,
relationship traversal):

```rust
#[xpath_fn("xfi:concept-balance($qname as xs:QName) as xs:string?")]
fn concept_balance(context: &DynamicContext, qname: Name) -> Option<String> {
    let dts = context.user_data::<Dts>()?;
    dts.concept(&qname)?.balance().map(|b| b.to_string())
}
```

Required: a typed-erased user-data slot on the dynamic context.
Read-only is good enough for v1; mutation can wait.

### 4.3 Per-evaluation context-stack functions

A formula evaluator processes one fact partition at a time. Some
`xfi:*` functions need to know *which fact is the current binding*
beyond what the XPath context node carries:

```rust
#[xpath_fn("xfi:c-equal($a as element(), $b as element()) as xs:boolean")]
fn c_equal(a: &Node, b: &Node) -> bool { ... }
```

This is structurally fine — the args carry everything needed —
but it reveals that the user-data slot also has to be **per-evaluation
mutable** in some scenarios. v1 punts on this: user-data is set at
`DynamicContext` build time and treated as read-only thereafter; the
consumer rebuilds a context per fact partition.

### 4.4 Doors we must not close

- **Multiple coexisting libraries in one process.** Two independent
  evaluators with different `xfi:*` registrations must not collide on a
  process-global.
- **Re-entrancy.** A user function invoking the interpreter recursively
  (e.g. `xff:uncovered-aspect` style) must work. The current
  `&mut Interpreter` parameter implies this is already the case for
  standard-library functions; we have to preserve it.
- **Future async / parallel evaluation.** The crate is single-threaded
  today, but the public types we add should not be intrinsically
  `Rc<...>` where `Arc<...>` would do.

## 5. Design overview

The shape of the change, top-down:

```
┌─────────────────────────────────────────────────────────────────┐
│  External crate                                                  │
│                                                                  │
│  use xee_xpath::function::{xpath_fn, wrap_xpath_fn};             │
│                                                                  │
│  #[xpath_fn("xfi:period-instant(...) as xs:date", context_first)]│
│  fn period_instant(node: &Node) -> NaiveDate { ... }             │
│                                                                  │
│  let mut sb = StaticContextBuilder::default();                   │
│  sb.namespaces([("xfi", XFI_NS)])                                │
│    .add_function(wrap_xpath_fn!(period_instant))                 │
│    .add_function(wrap_xpath_fn!(c_equal))                        │
│    ...;                                                          │
│                                                                  │
│  let mut db = DynamicContextBuilder::default();                  │
│  db.user_data(Arc::new(dts));                                    │
└─────────────────────────────────────────────────────────────────┘
```

Five changes make this work:

1. **Per-context `StaticFunctions`.** Drop the `&'static` reference.
   `StaticContext` owns an `Arc<StaticFunctions>`. The built-in
   library is loaded into every new `StaticContextBuilder` by default
   (so existing code keeps working with no source changes).
2. **`add_function` / `add_functions` on `StaticContextBuilder`.**
   Accepts `StaticFunctionDescription`s; merges them into the
   builder's pending function list. The list is consumed when
   `.build()` constructs the final `StaticFunctions` table.
3. **User-data slot on `DynamicContext`.** A typed-erased
   `Option<Arc<dyn Any + Send + Sync>>` field with a
   `user_data::<T>()` typed accessor. Set via `DynamicContextBuilder`.
4. **Public macro re-exports.** `xee_xpath::function::xpath_fn` and
   `xee_xpath::function::wrap_xpath_fn`. Macros' code-gen uses
   absolute paths under `::xee_xpath::function::*`, freeing them
   from `crate::` assumptions.
5. **Signature-namespace plumbing.** Signature parsing is deferred
   to `.build()` time, against the namespaces the builder holds at
   that point. Builder method order doesn't matter; `.build()`
   returns `Result<StaticContext, BuildError>` to surface bad
   signatures. See §10.1 for the rationale.

Constraint B (bytecode IDs) is resolved by deciding that the
`StaticFunctions` table is **frozen at `StaticContext::build` time**;
the resulting `Arc<StaticFunctions>` is captured by the compiled
`Program` and the ID indexes into that snapshot, not into any global.

## 6. Public API surface

### 6.1 `xee-xpath` re-exports

Add to `xee-xpath/src/lib.rs`:

```rust
pub mod function {
    pub use xee_xpath_macros::xpath_fn;
    pub use xee_interpreter::wrap_xpath_fn;
    pub use xee_interpreter::function::{
        StaticFunctionDescription,
        // and the inner types the macro-generated code needs to name:
        StaticFunctionType,
    };
    // re-exports of the types that user function bodies need:
    pub use xee_interpreter::context::DynamicContext;
    pub use xee_interpreter::interpreter::Interpreter;
    pub use xee_interpreter::sequence::Sequence;
    pub use xee_interpreter::error::{Error, Result};
}
```

This is the *only* import path public consumers need to know.

### 6.2 `StaticContextBuilder::add_function`

```rust
impl<'a> StaticContextBuilder<'a> {
    /// Register a single XPath function for use by expressions
    /// compiled with this static context.
    ///
    /// The function's signature is parsed when [`Self::build`] is
    /// called, against the namespaces the builder holds at that
    /// point — builder method order does not matter. A malformed
    /// signature surfaces as a [`BuildError`] from `.build()`.
    ///
    /// Re-registering the same `(name, arity)` overrides the
    /// earlier registration.
    pub fn add_function(
        &mut self,
        description: StaticFunctionDescription,
    ) -> &mut Self;

    /// Register multiple XPath functions at once.
    pub fn add_functions(
        &mut self,
        descriptions: impl IntoIterator<Item = StaticFunctionDescription>,
    ) -> &mut Self;

    /// Exclude the built-in standard library from this static
    /// context. By default, every static context includes the full
    /// `fn:*` / `xs:*` / `math:*` / `map:*` / `array:*` library.
    ///
    /// Use only when you want to expose a deliberately restricted
    /// XPath dialect (e.g. for sandboxed expressions). Most consumers
    /// should leave this alone.
    pub fn exclude_default_library(&mut self) -> &mut Self;
}
```

The `exclude_default_library` toggle is included from the start
because the formula-evaluator use case benefits from being able to
ban `fn:doc` / `fn:collection` / `fn:current-date` etc. for
deterministic evaluation. But it's a power-user feature: the docs
should steer ordinary consumers away.

### 6.3 `DynamicContextBuilder::user_data`

```rust
impl<'a> DynamicContextBuilder<'a> {
    /// Attach application-specific state to the dynamic context.
    /// Extension functions registered via
    /// [`StaticContextBuilder::add_function`] can recover it through
    /// [`DynamicContext::user_data::<T>()`].
    ///
    /// The slot is single-typed per context: calling this twice
    /// overwrites the earlier value. If you need multiple kinds of
    /// state, wrap them in a single struct.
    pub fn user_data<T>(&mut self, value: Arc<T>) -> &mut Self
    where
        T: Any + Send + Sync;
}
```

Reader access:

```rust
impl<'a> DynamicContext<'a> {
    /// Retrieve the user-data value attached at build time, if any
    /// was provided and its type matches.
    pub fn user_data<T: Any + Send + Sync>(&self) -> Option<&T>;
}
```

`Send + Sync` is a forward-looking bound — the crate is single-threaded
today but the type is cheap to require now and expensive to add later.

### 6.4 User function authoring shape

The current `#[xpath_fn]` keeps its existing argument-injection
behaviour:

```rust
use xee_xpath::function::{
    xpath_fn, DynamicContext, Interpreter, Sequence,
};

#[xpath_fn("xfi:concept-balance($qname as xs:QName) as xs:string?")]
fn concept_balance(
    context: &DynamicContext,           // auto-injected if declared; omit if unused
    qname: xee_name::Name,
) -> Option<String> {
    let dts = context.user_data::<Dts>()?;
    dts.concept(&qname).and_then(|c| c.balance().map(str::to_owned))
}
```

`context` and `interpreter` injection works as it does internally
(see `xee-xpath-macros/src/wrapper.rs:67-89`); no source-level changes
to the macro's user contract.

### 6.5 What stays private

- `StaticFunctionType` (the bare fn-pointer alias) is exposed because
  the macro-generated code refers to it, but it's marked
  `#[doc(hidden)]` — not part of the stable surface.
- `StaticFunctions` (the registry struct) stays crate-internal. The
  builder/runtime never hand it out directly.
- `StaticFunctionId` stays crate-internal. External consumers never
  see opaque integer IDs.
- The `FunctionKind`/`FunctionRule` enums stay crate-internal. The
  macro keyword (`context_first`, `context_last`, etc.) is the
  user-facing surface.

## 7. Internal changes

### 7.1 Drop the singleton

`xee-interpreter/src/context/static_context.rs`:

- Replace the existing `static STATIC_FUNCTIONS: LazyLock<...>` with
  the `OnceLock<Arc<StaticFunctions>>` cache fronted by a
  `default_library()` helper (see §9 Phase 1 for the helper shape).
- Replace the `&'static function::StaticFunctions` field with
  `functions: Arc<function::StaticFunctions>`.
- `StaticContext::new` accepts an `Arc<StaticFunctions>` as an
  additional parameter (or via a separate constructor for the
  built-in case).
- `From<XPathParserContext>` is either removed or upgraded to
  also take the function table.

`StaticContextBuilder::build` constructs the `StaticFunctions` table
from (a) the built-in library (unless excluded) and (b) any
descriptions added via `add_function`, parses each signature against
the builder's namespaces, and wraps it in `Arc`.

### 7.2 Per-builder pending function list

`StaticContextBuilder` gains:

```rust
pending_functions: Vec<PendingFunction>,
include_default_library: bool,
```

where `PendingFunction` holds the raw signature string and the
`StaticFunctionType` pointer, parsed at `.build()` time against
the builder's namespaces. Deferred parsing is what makes user-
prefix signatures (`xfi:c-equal(...)`) work without forcing the
consumer to thread namespaces through the macro; see §10.1 for the
rationale.

### 7.3 Macro path repointing

`xee-xpath-macros/src/wrapper.rs` currently emits code with paths
like `crate::function::StaticFunctionType`,
`crate::context::DynamicContext`, etc. These assume the user is
inside `xee-interpreter`.

The fix: emit absolute paths through a configurable crate root.
Cleanest is the standard `$crate` trick used by declarative macros,
adapted to proc-macros: a small helper at the top of the generated
code resolves the right crate root via a re-export.

```rust
// generated code now references:
::xee_xpath::function::__macro_support::StaticFunctionType
::xee_xpath::function::__macro_support::DynamicContext
::xee_xpath::function::__macro_support::Interpreter
::xee_xpath::function::__macro_support::Sequence
::xee_xpath::function::__macro_support::Error
```

`xee_xpath::function::__macro_support` is a `#[doc(hidden)]` module
that re-exports the inner types. The internal library code keeps
using the `crate::` paths via a parallel re-export inside
`xee-interpreter` (or via the same path through a workspace
re-export).

Alternative: keep two macros, one internal (`crate::` paths) and one
public (`xee_xpath::function::` paths). Loses DRY but is less
invasive. See Open Question 10.5.

### 7.4 `DynamicContext` user-data field

`xee-interpreter/src/context/dynamic_context.rs` grows one field:

```rust
user_data: Option<Arc<dyn Any + Send + Sync>>,
```

and `DynamicContext::user_data::<T>()` is a thin `downcast_ref`
wrapper. `DynamicContextBuilder::user_data` stores it; the field is
purely additive — the existing `DynamicContext::new` constructor
grows by one parameter, but no other interpreter code path has to
know about it.

### 7.5 `Program` and bytecode

`Program` captures an `Arc<StaticFunctions>` at compile time (cloning
the `Arc` from the `StaticContext` that compiled it). Bytecode IDs
stay valid for the program's lifetime. No bytecode-format change.

The existing `StaticContext::function_by_id` /
`function_id_by_name` / `function_id_by_internal_name`
(`static_context.rs:127-150`) keep their shape; their internal
implementation reaches through `self.functions` (the `Arc`) rather
than through a process-wide static.

### 7.6 Signature parsing with extended namespaces

`StaticFunctionDescription::new`
(`static_function.rs:84-104`) currently calls
`ast::Signature::parse(signature, namespaces)`.

For external functions, the namespaces passed in must include the
prefixes the user has bound on the builder. This means the **builder
owns signature parsing**, not the macro. The macro stores the raw
signature string (which it already does — see
`wrapper.rs:21, 40`); the builder, at `.build()` time, parses each
pending signature against the merged namespace map.

For internal functions, this is a no-op semantic change: they were
already parsed against `DEFAULT_NAMESPACES`, which becomes a subset
of every external namespace set.

## 8. Backward compatibility and SemVer

### 8.1 What does not break

- The `#[xpath_fn]` macro keeps its user-facing syntax. All existing
  internal-library function definitions compile unchanged.
- `StaticContext::default()`, `StaticContextBuilder::default()`,
  `Queries::default()` continue to produce a context with the full
  built-in library.
- The public `xee-xpath` API (`Queries`, `Query`, `Documents`, etc.)
  keeps its function-related surface — function lookup is still by
  qname-and-arity, conducted by the parser.

### 8.2 What breaks (minor-version)

- Any external code that names `xee_interpreter::context::StaticContext`
  directly and constructs it via `From<XPathParserContext>` (the
  current impl). This conversion either grows the function-table
  parameter or is replaced with a builder method. The grep on the
  workspace shows zero non-test uses outside `xee-interpreter` itself,
  so churn is contained.
- The `StaticContextBuilder::build` return type stays
  `StaticContext`, but the internal layout differs.

### 8.3 What we deliberately do not promise

- The shape of `StaticFunctionDescription` is **not** stable API.
  Consumers should produce it only via `wrap_xpath_fn!`. The
  struct itself is `#[doc(hidden)]`.
- The `__macro_support` re-export module is `#[doc(hidden)]` and
  may change between minor versions in lockstep with the macro.

## 9. Implementation plan — phased PRs

Each phase is a single small upstream-mergeable PR. Phases land in
order; later phases can be revised based on review feedback on
earlier ones.

### Phase 1 — desingleton the function registry (no user-visible change)

Change `StaticContext` to hold `Arc<StaticFunctions>` instead of
`&'static StaticFunctions`. Back the default library with a
process-wide cache:

```rust
fn default_library() -> Arc<StaticFunctions> {
    static CACHE: OnceLock<Arc<StaticFunctions>> = OnceLock::new();
    CACHE.get_or_init(|| Arc::new(StaticFunctions::new())).clone()
}
```

Every `StaticContext` built without custom functions calls
`default_library()` and gets a cheap `Arc::clone` of the same
underlying allocation. The user-visible perf difference vs today's
`&'static` is one atomic refcount bump per `StaticContext::new` —
nothing in the hot path, and the `StaticFunction` values themselves
are still shared across the entire process.

`StaticContextBuilder::build()` calls `default_library()` and stores
the resulting `Arc`. Only when Phase 2 introduces `add_function` does
the builder actually allocate a fresh `StaticFunctions` (built from
the cached library plus the pending user descriptions).

All existing call sites adapt mechanically: `&'static
StaticFunctions` becomes `&StaticFunctions` (auto-deref'd from the
`Arc`) at the read sites; `function_by_id`, `function_id_by_name`,
etc. keep their signatures.

This PR is correctness-only: no semantic changes, no API changes,
internal layout shifts. Reviewable in isolation. Establishes the
foundation for Phases 2–4.

**Risk:** any code that captured the `&'static StaticFunctions`
reference into a longer-lived structure (none today, but worth a
grep) would need to switch to holding an `Arc` clone.

**Send/Sync note for reviewers.** The new `Arc<StaticFunctions>`
preserves the existing Send/Sync story exactly: `StaticFunction`
values are bare fn-pointers, which are `Send + Sync`, so
`Arc<StaticFunctions>` carries the same auto-trait shape that
`&'static StaticFunctions` had. Phase 1 is Send/Sync-neutral; the
question of whether `DynamicContext` becomes `Send` lives entirely
in Phase 4 and §10.3.

**LOC estimate:** ~150 lines diff, all in `xee-interpreter`.

### Phase 2 — `add_function` builder API + user-prefix signature parsing

`StaticContextBuilder::add_function` and `add_functions`. Builder
collects pending descriptions; `.build()` merges built-in + pending
into the final `StaticFunctions`. Signature parsing moves to
`.build()` time.

**Integration-test scope.** The PR must include at least:

1. A toy registration smoke (any-signature external function called
   from an XPath expression — covers the happy path).
2. A **sequence-returning** external function (signature like
   `ext:nodes() as element()*` returning multiple items). The
   existing library has functions of this shape; the goal is to
   pin the contract for external authors, since all our examples
   so far show scalar / option returns.
3. A **re-entrant** external function (one that invokes the
   injected `&mut Interpreter` to evaluate a sub-expression).
   Internal library functions already do this; this test confirms
   the same path works from outside the crate. Highest-uncertainty
   path in the design (§12).

**Risk:** signature parse errors are now deferred until
`.build()` — they used to surface at static-init time. Need a clean
error variant (`StaticContextBuildError::FunctionSignature { ... }`)
that fingers the offending registration. Builder should return
`Result<StaticContext, BuildError>` going forward; today `.build()`
is infallible.

**LOC estimate:** ~300 lines, plus tests.

### Phase 3 — macro path repointing + `xee_xpath::function` re-exports

`xee-xpath-macros` emits absolute paths through
`::xee_xpath::function::__macro_support`. `xee-xpath` adds the
`function` module with re-exports. End-to-end smoke test: an
external integration test crate in the workspace registers a
toy function and evaluates an expression that calls it.

**Risk:** internal library code (`xee-interpreter/src/library/*.rs`)
still uses the macro with `crate::` paths in mind. Two options
(see Open Question 10.5). The simplest: the macro emits paths
via a configurable root and a workspace-internal feature flag.
Internal library uses one root; external consumers use another.

**LOC estimate:** ~400 lines including the integration test crate.

### Phase 4 — user-data slot on `DynamicContext`

Add the `user_data` field and builder method.

**Independent of Phases 2 and 3.** This PR touches
`DynamicContext` / `DynamicContextBuilder` only; it has no
dependency on the function-registration changes from Phase 2 or
the macro re-export from Phase 3. It can land in any order
relative to those, including in parallel — useful for consumers
that need to start integrating against the user-data slot before
the full extension-function surface is upstream. The only
ordering constraint is that consumers writing extension
functions in practice want Phases 2 + 3 + 4 all available before
they can ship.

**LOC estimate:** ~80 lines.

### Phase 5 — `exclude_default_library` toggle (optional)

Power-user feature. Defer if review feedback says so. Probably 50
lines.

### Phase 6 — docs and migration aids

A top-level rustdoc page on `xee_xpath::function` walking through
the toy example. README updates to retract the "support for
extension functions needs to be created" line.

## 10. Open questions

The questions below are the ones we need to resolve before writing
phase-2 / phase-3 code. Each has a recommended answer and the
reasoning; flagging them so we can revisit before they harden.

### 10.1 Eager vs deferred signature parsing

The macro currently stores the signature as a string and reparses
at registration time. Should the `StaticContextBuilder` retain
that approach (deferred — parse at `.build()` time) or eager-parse
each `add_function` call?

**Recommendation: deferred.** Eager parsing forces the user to
register namespaces *before* functions, which is brittle. Deferred
parsing lets the builder methods be called in any order, and the
single parse pass at `.build()` time produces a coherent error
context. Cost: `.build()` becomes fallible.

### 10.2 How a user function reaches `user_data`

Three candidates:

- (a) `context.user_data::<T>()` on `&DynamicContext`.
- (b) `interpreter.user_data::<T>()` on `&mut Interpreter`.
- (c) Both.

**Recommendation: (a).** `DynamicContext` is the natural owner;
`Interpreter` is execution machinery and shouldn't accumulate
context-shaped state. The macro already supports a `context:
&DynamicContext` injected first parameter, so the ergonomics are
already there.

Followup: should `xpath_fn` learn a `user_data` keyword that injects
the typed reference automatically? `#[xpath_fn("...", user_data)]
fn foo(dts: &Dts, ...)`. Probably yes in a later iteration, not the
first cut; defer the keyword design until two consumers want it.

### 10.3 `Send + Sync` on user data — yes or no?

`xot::Xot` is `!Send` today (uses `Rc` internally — confirm before
finalising). If the dynamic context becomes `!Send` anyway, requiring
`Send + Sync` on user data buys us nothing. If we expect Xot to gain
`Send` in some future version, the bound is worth keeping.

**Recommendation: require `Send + Sync` from the start.** Cheap now,
expensive to add later. If a consumer genuinely needs non-Send state,
they wrap it in `Mutex` or accept that their evaluator is
single-threaded.

### 10.4 Higher-order extension functions

The macro currently has an `anonymous_closure` `FunctionKind` for
internal use (`xee-interpreter/src/function/static_function.rs:36-50`).
Should external consumers be able to register functions that take
`function(...)` arguments (HOFs) or return them?

**Recommendation: not in v1.** The XBRL `xfi:*` library has zero HOFs.
Locking down only first-order extension functions for v1 is a
conservative choice that doesn't constrain the macro contract; a
later iteration can lift the restriction once we've seen what shapes
consumers actually want.

### 10.5 One macro or two?

The internal library uses `crate::function::*` paths today. If the
public macro emits `::xee_xpath::function::*` paths, the same macro
cannot serve both unless we feature-flag the crate root.

**Recommendation: one macro with a feature-flagged crate root.**
`xee-xpath-macros` gains a `internal` Cargo feature; when active,
generated paths use `crate::function::*`, otherwise
`::xee_xpath::function::__macro_support::*`. `xee-interpreter`'s
dep on `xee-xpath-macros` enables the feature; downstream consumers
do not. This keeps the macro DRY.

Alternative: two macros (`xpath_fn` public, `xpath_fn_internal`
private). More code, less risk of accidentally exposing internal
paths.

### 10.6 Error model for user function errors

Internal functions return `error::Result<Sequence>` where `Error` is
the crate's full enum, including XPath spec error codes (`FOER0000`,
etc.) and `error::ApplicationError` for user-supplied codes
(`xee-interpreter/src/library/fn_.rs:32-77`).

External functions presumably want to raise *their own* error codes —
both XPath-style QName errors (e.g. `xfie:invalidContext`) and free-
form messages.

**Recommendation: expose `error::ApplicationError::new(qname, message)`
through the public re-export.** Consumers raise
`Err(Error::Application(Box::new(ApplicationError::new(qname,
"...").into())))`. Verbose but explicit. A helper builder can land
later: `Error::user(qname, "...")`.

### 10.7 Re-registering `(name, arity)` — error or silent override?

The current internal registry treats duplicate `(name, arity)` as a
single insertion (later wins, since the `HashMap::insert` overwrites
— see `static_function.rs:286-295`).

**Recommendation: silent override, document loudly.** Lets consumers
deliberately replace built-in functions for testing / sandboxing.
Combined with `exclude_default_library`, this is enough to build a
fully custom dialect. The duplicate-detection-as-error story can be
added later as a builder option (`strict_registration(true)`) if real
users hit foot-guns.

### 10.8 Function ID stability across multiple `StaticContext`s built from the same builder

If a consumer calls `builder.build()` twice, do the two resulting
`StaticContext`s share an `Arc<StaticFunctions>` or each get their
own?

**Recommendation: each builds its own.** Cheaper than tracking
whether the builder has been mutated since the last build. The
built-in library is cached, so repeated builds with the default
configuration are fast. The cost is paid only when the consumer
registers custom functions, which they do rarely.

## 11. Alternatives considered

### 11.1 Runtime closure registration

`StaticContextBuilder::add_function_dyn(signature_string, closure)`
where `closure` is `Box<dyn Fn(&DynamicContext, &mut Interpreter,
&[Sequence]) -> Result<Sequence, Error>>`. Avoids the macro
indirection entirely.

**Rejected for v1.** The macro is the existing developer
experience; matching it for external consumers is more valuable
than offering a closure escape hatch. A runtime API can be added
later in parallel — closures and macro-generated functions can
share `StaticFunctionDescription`.

### 11.2 Generic-typed `DynamicContext<U>`

Parameterise the dynamic context on the user-data type. Type-safe;
no `Any` cast. But the type parameter infects every public API
that names a context — `Query::execute`, every accessor, every
trait — and forces consumers that don't want user-data to write
`DynamicContext<()>` everywhere.

**Rejected.** The cost in API surface is much larger than the
benefit. Type-erased `Any` is the right tradeoff for a once-per-
evaluation lookup.

### 11.3 Function table as a separate handle parameter

Pass the function table to `Queries::new` directly:
`Queries::new(static_context_builder, function_library)`. Keeps
the builder simple but introduces a second concept (the library)
parallel to the existing builder.

**Rejected.** The builder is already the configuration surface;
adding functions to it is the principle-of-least-surprise choice.

### 11.4 Macro-free public API

Skip the macro re-export entirely; consumers write
`StaticFunctionDescription` by hand or via a runtime builder.

**Rejected.** Hand-writing the wrapper closure that
`#[xpath_fn]` generates is ~50 lines per function and includes
sequence-type conversion code. The DX would push consumers back
to forking the engine.

### 11.5 Split tables — built-in singleton plus secondary external registry

Keep `StaticContext`'s `&'static StaticFunctions` exactly as it is
today. Add a parallel `ExternalFunctionRegistry` owned per builder
/ per program. The compiler does a two-step lookup at function-name
resolution: try the built-in singleton first, fall back to the
external registry.

What this buys:

- The `&'static` singleton survives untouched — Phase 1 of §9 goes
  away. Smaller upstream diff for that piece.
- Built-in spec functions can never be shadowed: `fn:count` always
  resolves to the spec implementation, by construction.
- A clean precedence story without needing the silent-override
  caveat (§10.7).
- One atomic refcount fewer per `StaticContext::new` (the unified
  design with `OnceLock` caching costs one `Arc::clone`; this costs
  zero).

What this costs:

- **Bytecode and IR grow a discriminant.** `StaticFunctionId(usize)`
  appears in 30+ places. The compiler bakes it into
  `ir::Const::StaticFunctionReference` (`xee-ir/src/ir.rs:76`) and
  `Instruction::StaticClosure` (`xee-ir/src/function_compiler.rs:421`).
  The interpret loop dispatches on it in four spots
  (`xee-interpreter/src/interpreter/interpret.rs:161, :654, :663,
  :676, :755`). With two tables the id needs a tag bit, an enum,
  or sibling IR/instruction variants — each a real edit across two
  crates.
- **`Function` value enum grows by a variant.** It is currently
  `Static | Inline | Map | Array` (`function_core.rs:35-40`) with a
  16-byte size assertion (`:43`). Adding `External` touches every
  pattern-match site and may bust the size assertion.
- **Two mental models for "what is a static function".** Reviewers,
  future contributors, and the rustdoc all carry that split forever.

**Rejected** in favour of the cached-unified design (one
`StaticFunctions` per context, default library shared via
`OnceLock<Arc<StaticFunctions>>` — see §9 Phase 1). The cached-
unified design captures most of the singleton-preservation benefit
(default-library storage is still one allocation, shared across all
default-configured contexts) without paying the IR / bytecode /
`Function`-enum tax. The remaining structural wins of split tables
— guaranteed non-shadowing of spec functions — is a semantic
choice that can be added as a lint or builder option (`strict_spec(
true)` rejecting overrides of names in well-known XPath namespaces)
without touching the IR.

## 12. Risks

- **Upstream review may want a different API shape.** This doc is
  the negotiation surface; we should publish the design as a draft
  upstream issue before Phase 2 lands, ideally with Phase 1
  already merged to demonstrate the technical groundwork.
- **`Send + Sync` may turn out to be wrong if `xot::Xot` stays
  `!Send`.** Cheap to relax (drop the bound); expensive to add.
- **`xfi:*` function bodies will surface needs we haven't
  anticipated.** Most likely: nested function calls (an `xfi:*`
  helper that itself invokes an XPath sub-expression). The
  `&mut Interpreter` injection already supports this for internal
  functions; the Phase 2 PR includes the integration test that
  exercises this from an external function (see §9 Phase 2).
- **Per-evaluation `DynamicContext` construction cost.** §4.3 punts
  per-partition state to "rebuild the dynamic context per partition".
  Real consumer workloads (e.g. a formula evaluator binding
  variables across hundreds–thousands of fact partitions per
  invocation) may make context construction hot enough to matter.
  Phase 2 should include a microbench of `DynamicContextBuilder::
  build` repeated against a fixed `Program`; if the cost is non-
  trivial, a `DynamicContextBuilder::clone` or a mutable
  variable-bindings API needs to come earlier than v2.

## 13. Out of this document

These belong in companion documents, written separately:

- The `xfi:*` library implementation itself — what subset to ship
  for v1, what `xfi:*` semantics to test against, how the DTS
  handle threads through. Lives in the downstream consumer, not in
  Xee.
- Pluggable XML data model (a non-Xot tree, or an adapter over a
  typed data model). Separate upstream contribution; orthogonal to
  extension functions.
- Pluggable `fn:doc` / `fn:collection` resolvers. Useful, but
  separate from extension-function registration.
- Performance characterisation of `Arc<StaticFunctions>` vs the
  current `&'static`. Almost certainly negligible (one extra
  reference-count bump per `StaticContext::new`); needs a microbench
  before Phase 1 merges.
