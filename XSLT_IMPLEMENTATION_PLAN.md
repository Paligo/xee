# XEE XSLT 3.0 Implementation Plan
## Path to 0 Errors in Test Suite

**Last Updated**: 2026-03-19

---

## Current Test Status

```
Total Tests:        14,595
Supported Tests:    14,595 (100%)
Passing Tests:      1,098
Filtered (Known):   13,497
Failed Tests:       0
Error Tests:        0
Wrong Error Tests:  0
```

**Goal**: Move all 13,497 filtered tests to passing.

---

## How to Run Tests

```bash
cd /Users/brillo/Repositories/xee/xee-testrunner

# Check regressions (only tests known to pass)
cargo run --release -- check ../vendor/xslt-tests/

# Run all tests
cargo run --release -- all ../vendor/xslt-tests/

# Run verbose
cargo run --release -- -v all ../vendor/xslt-tests/

# Update filters after fixes
cargo run --release -- update ../vendor/xslt-tests/

# Verify no regressions
cargo run --release -- check ../vendor/xslt-tests/
```

---

## Outstanding Implementation Issues with Difficulty Scores

### ⭐ PRIORITY 1: CRITICAL BLOCKERS (Unblocks ~70% of tests)

| Issue | Category | Impact | Tests | Score | Effort | Notes |
|-------|----------|--------|-------|-------|--------|-------|
| **Mode Declaration Parsing** | AST | 110+ function tests fail | 110+ | **1** | 1-2 days | Parse `xsl:mode` declarations. Simple AST extension. |
| **Mode Support in Apply-Templates** | Compiler | Template dispatch broken | 500+ | **2** | 3-5 days | Compute and pass mode parameter through template calls. |
| **Call-Template Instruction** | IR Support | Named templates not callable | 300+ | **2** | 2-3 days | Add CallTemplate IR node, execution handler. |
| **Template Matching System** | Compiler | Pattern matching incomplete | 200+ | **2** | 4-7 days | Priority calc, pattern variables, rooted paths. |
| **Import/Include Subsystem** | Architecture | 42+ tests 100% fail | 600+ | **3** | 2-3 weeks | Stylesheet composition, precedence, symbol tables. *Most complex* |
| **Apply-Imports Instruction** | Compiler | Named template inheritance blocked | 100+ | **3** | 1-2 weeks | Depends on import system. |
| **Next-Match Instruction** | Compiler | Template rule chaining broken | 80+ | **2** | 1 week | Depends on mode + template system. |

---

### 🔸 PRIORITY 2: HIGH-VALUE FEATURES (Unblocks ~15% of tests)

| Issue | Category | Impact | Tests | Score | Effort | Notes |
|-------|----------|--------|-------|-------|--------|-------|
| **Parameter System (xsl:param)** | AST/Compiler | 200+ tests fail | 200+ | **2** | 3-4 days | Global parameters, parameter binding. |
| **Variables (xsl:variable)** | AST/Compiler | 150+ tests | 150+ | **2** | 2-3 days | Scope, compile-time variables. |
| **Sorting (xsl:sort)** | Compiler | 120+ tests | 120+ | **2** | 2-3 days | Sort algorithm, XPath key extraction. |
| **Perform-Sort Instruction** | Compiler | 60+ tests | 60+ | **1** | 2 days | Built on xsl:sort. |
| **Pattern Fixes** | Compiler | 90+ tests | 90+ | **2** | 2-3 days | Variables in patterns, rooted paths, axes. |
| **Attribute Sets** | AST/Compiler | 80+ tests | 80+ | **2** | 2-3 days | Parse, store, apply xsl:attribute-set. |
| **Output Method** | Compiler | 70+ tests | 70+ | **2** | 2-3 days | Output format options, serialization. |
| **Analyze-String** | Compiler | 50+ tests | 50+ | **1** | 1-2 days | Integrate regexml (library ready). |

---

### 🟡 PRIORITY 3: MEDIUM FEATURES (Unblocks ~8% of tests)

| Issue | Category | Impact | Tests | Score | Effort | Notes |
|-------|----------|--------|-------|-------|--------|-------|
| **Key Declaration** | AST/Compiler | 60+ tests | 60+ | **2** | 2-3 days | xsl:key definition, key() function. |
| **Result-Document** | Compiler | 50+ tests | 50+ | **2** | 2-3 days | Multiple output documents. |
| **Character Map** | AST/Compiler | 40+ tests | 40+ | **1** | 1-2 days | Simple mapping table. |
| **Copy/Element Construction** | Compiler | 60+ tests | 60+ | **2** | 2-3 days | Missing attributes (copy-namespaces, use-attribute-set). |
| **Error Handling (try/catch)** | IR/Interpreter | 40+ tests | 40+ | **2** | 2-3 days | Error control flow. |
| **Iterate/Break** | IR/Interpreter | 35+ tests | 35+ | **2** | 2-3 days | Loop control. |
| **For-Each-Group** | Compiler | 30+ tests | 30+ | **2** | 2-4 days | Grouping logic. |
| **Message Instruction** | Compiler | 25+ tests | 25+ | **1** | 1 day | Diagnostic output. |
| **Assert Instruction** | Compiler | 20+ tests | 20+ | **1** | 1 day | Assertions. |

---

### 🟠 PRIORITY 4: LOWER-VALUE FEATURES (Unblocks ~4% of tests)

| Issue | Category | Impact | Tests | Score | Effort | Notes |
|-------|----------|--------|-------|-------|--------|-------|
| **Namespace Alias** | AST/Compiler | 15+ tests | 15+ | **2** | 1-2 days | Namespace mapping. |
| **Number Instruction** | Compiler | 15+ tests | 15+ | **2** | 2 days | Awaits xee-format crate. |
| **Decimal Format** | Compiler | 10+ tests | 10+ | **2** | 1-2 days | Awaits xee-format crate. |
| **Merge** | Compiler | 10+ tests | 10+ | **3** | 2-3 days | Complex multi-source merge. |
| **Evaluate** | Interpreter | 8+ tests | 8+ | **3** | 1-2 days | Dynamic expression evaluation. |
| **On-Empty/On-Non-Empty** | Compiler | 5+ tests | 5+ | **1** | 1 day | Fallback sequences. |
| **Context-Item/Global-Context-Item** | Compiler | 5+ tests | 5+ | **2** | 1 day | Context variables. |
| **Map/Map-Entry** | Compiler | 3+ tests | 3+ | **2** | 1 day | Map data construction. |

---

### 🔴 PRIORITY 5: STREAMING & ADVANCED (Deprioritized)

| Issue | Category | Impact | Tests | Score | Effort | Notes |
|-------|----------|--------|-------|-------|--------|-------|
| **Accumulator** | IR/Interpreter | <5 tests | <5 | **3** | 1-2 weeks | Streaming support. Deprioritized. |
| **Fork** | IR/Interpreter | <5 tests | <5 | **3** | 1-2 weeks | Streaming support. Deprioritized. |
| **Schema Import/Validation** | IR/Interpreter | <10 tests | <10 | **3** | 2-3 weeks | Deep schema integration. |
| **Package/Use-Package** | Architecture | <10 tests | <10 | **3** | 1-2 weeks | Module system. Depends on import. |

---

## Implementation Difficulty Legend

- **Score 1 (Easy)**: 1-2 days, localized changes, minimal dependencies
  - *Examples*: Mode parsing, perform-sort, analyze-string, character-map, message, assert
  
- **Score 2 (Medium)**: 2-7 days, moderate complexity, some dependencies
  - *Examples*: Template system fundamentals, parameters, variables, sorting, output methods, pattern fixes
  
- **Score 3 (Difficult)**: 1-3 weeks+, high complexity, many dependencies or architectural
  - *Examples*: Import/include subsystem, streaming, schema support, packages, merge

---

## Recommended Implementation Roadmap

### Phase 1: Mode & Template Foundation (1-2 weeks)
**Unblocks**: ~500 tests

1. Parse `xsl:mode` declarations (Score 1)
2. Implement mode parameter passing (Score 2)
3. Fix template matching system (Score 2)
4. Implement `xsl:call-template` (Score 2)

### Phase 2: Parameters & Variables (3-5 days)
**Unblocks**: ~350 tests

5. Parameter system (`xsl:param`, `xsl:with-param`) (Score 2)
6. Variables (`xsl:variable`) (Score 2)

### Phase 3: Common Instructions (4-7 days)
**Unblocks**: ~300 tests

7. Sorting (`xsl:sort`, `xsl:perform-sort`) (Score 2)
8. Pattern fixes (variables, rooted paths) (Score 2)
9. Attribute sets (`xsl:attribute-set`) (Score 2)

### Phase 4: Output & Analysis (2-4 days)
**Unblocks**: ~120 tests

10. Output methods (Score 2)
11. Analyze-string (Score 1)
12. Character maps (Score 1)

### Phase 5: Advanced Features (2-4 weeks)
**Unblocks**: ~100+ tests

13. Import/include subsystem (Score 3) ← Major architectural effort
14. For-each-group, merge, etc. (Score 2)
15. Error handling, iteration (Score 2)

### Phase 6: Refinements & Edge Cases (1-2 weeks)
**Unblocks**: ~50+ tests

16. Copy/element construction details
17. Keys, result-document
18. Namespace alias, number, decimal-format
19. Messages, assertions
20. Context items

### Phase 7: Optional Advanced (Deprioritized)
- Streaming (accumulator, fork)
- Schema integration
- Package/use-package

---

## Key Code Locations

| Component | Location | Purpose |
|-----------|----------|---------|
| Main Compiler | `xee-xslt-compiler/src/ast_ir.rs` | AST → IR compilation (main implementation work) |
| Priority System | `xee-xslt-compiler/src/priority.rs` | Template match priority (has 4 panics needing fixes) |
| AST Parsing | `xee-xslt-ast/src/instruction.rs` | XSLT element parsing (8+ TODOs) |
| Attributes | `xee-xslt-ast/src/attributes.rs` | Attribute validation (gaps) |
| Tests | `xee-xslt-compiler/tests/test_xslt.rs` | Unit tests for compiler |
| Conformance | `conformance/xslt.md` | Master tracking doc |
| Test Filters | `vendor/xslt-tests/filters/` | Known failures to update |

---

## Testing Workflow

After implementing each feature:

1. **Run verbose tests** to see what passes:
   ```bash
   cargo run --release -- -v all ../vendor/xslt-tests/
   ```

2. **Update filters** to move now-passing tests out of filtered:
   ```bash
   cargo run --release -- update ../vendor/xslt-tests/
   ```

3. **Verify no regressions**:
   ```bash
   cargo run --release -- check ../vendor/xslt-tests/
   ```
   Should show: `Failed: 0 Error: 0 WrongE: 0`

---

## Useful Test Commands

```bash
# Run specific test category
cargo run --release -- -v all ../vendor/xslt-tests/tests/decl/template/

# Run single test file
cargo run --release -- -v all ../vendor/xslt-tests/tests/decl/template/_template-test-set.xml

# Run tests matching pattern
cargo run --release -- -v all ../vendor/xslt-tests/tests/decl/template/_template-test-set.xml template-005

# See test names
cargo run --release -- -v check ../vendor/xslt-tests/ 2>&1 | grep "FAIL\|ERROR"
```

---

## Notes

- **Import/Include**: This is the most complex feature (~1000+ tests depend on it). Should be tackled after template foundations are solid.
- **Streaming**: Deprioritized (~5% of tests). Can be added later if needed.
- **Mode**: Quick win (Score 1) but blocks many tests. Do this first.
- **Pattern fixes**: Medium effort but high impact. Do mid-Phase 1.
- **Error handling**: Some tests just need basic try/catch IR nodes.
- **Format crate**: `number` and `decimal-format` await `xee-format` crate availability.

---

## Success Metrics

- Phase 1 complete: 2,000+ tests passing (from 1,098)
- Phase 2 complete: 2,500+ tests passing
- Phase 3 complete: 3,500+ tests passing
- Phase 4 complete: 4,500+ tests passing
- Phase 5 complete: 5,500+ tests passing
- All phases: 14,595 passing (100%)

