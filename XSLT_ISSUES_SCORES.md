# XEE XSLT Implementation Issues - Quick Reference

## Issue Scoring Guide

| Score | Time | Complexity | Example |
|-------|------|-----------|---------|
| **1** ⭐ | 1-2 days | Simple, localized | Parse mode, analyze-string, character-map |
| **2** ⭐⭐ | 3-7 days | Moderate, some dependencies | Template system, parameters, sorting |
| **3** ⭐⭐⭐ | 1-3 weeks+ | Complex, architectural | Import/include system, streaming |

---

## All Outstanding Issues by Priority & Score

### IMMEDIATE WINS (Score 1 - Do First!)

| Issue | Tests | Block #1 | Block #2 | Time |
|-------|-------|---------|---------|------|
| Parse `xsl:mode` declarations | 110+ | Function tests | Template modes | **2h** |
| `xsl:analyze-string` (regexml ready) | 50+ | Regex tests | Streaming | **6h** |
| `xsl:character-map` | 40+ | Output tests | - | **6h** |
| `xsl:message` | 25+ | Error tests | - | **4h** |
| `xsl:assert` | 20+ | Validation tests | - | **4h** |
| `xsl:on-empty` / `xsl:on-non-empty` | 5+ | Iteration tests | - | **4h** |
| **SUBTOTAL** | **250+** | | | **26h** |

---

### CORE FEATURES (Score 2 - Medium Effort)

| Issue | Tests | Depends On | Time | Priority |
|-------|-------|-----------|------|----------|
| Mode parameter passing | 300+ | Mode parsing ↑ | **4d** | 1 |
| Template matching/priority | 200+ | Mode passing ↑ | **4d** | 1 |
| `xsl:call-template` | 300+ | Template system ↑ | **3d** | 1 |
| Pattern fixes (variables, rooted) | 90+ | Template system ↑ | **3d** | 1 |
| `xsl:param` / `xsl:with-param` | 200+ | Template system ↑ | **3d** | 2 |
| `xsl:variable` | 150+ | Parameter system ↑ | **3d** | 2 |
| `xsl:sort` | 120+ | Compiler support | **3d** | 2 |
| `xsl:perform-sort` | 60+ | `xsl:sort` ↑ | **2d** | 2 |
| `xsl:output` | 70+ | IR support | **3d** | 2 |
| `xsl:attribute-set` | 80+ | Attribute handling | **3d** | 2 |
| `xsl:key` | 60+ | Dynamic lookup | **3d** | 3 |
| `xsl:result-document` | 50+ | Output routing | **3d** | 3 |
| Copy/element attribute fixes | 60+ | Node construction | **3d** | 3 |
| `xsl:for-each-group` | 30+ | Grouping logic | **4d** | 3 |
| `xsl:try` / `xsl:catch` | 40+ | Error control flow | **3d** | 3 |
| `xsl:iterate` / `xsl:break` | 35+ | Loop control | **3d** | 3 |
| `xsl:namespace-alias` | 15+ | Namespace mapping | **2d** | 4 |
| `xsl:number` | 15+ | xee-format | **2d** | 4 |
| `xsl:decimal-format` | 10+ | xee-format | **2d** | 4 |
| `xsl:map` / `xsl:map-entry` | 3+ | Type system | **2d** | 4 |
| `xsl:merge` | 10+ | Source merging | **3d** | 4 |
| Context items | 5+ | Variable scoping | **2d** | 4 |
| **SUBTOTAL** | **1,493+** | | **67d** | |

---

### ARCHITECTURAL COMPLEXITY (Score 3 - Big Projects)

| Issue | Tests | Dependency Chain | Time | Notes |
|-------|-------|------------------|------|-------|
| **Import/Include subsystem** | 1,000+ | None (root) | **2-3 weeks** | MOST COMPLEX - Blocks apply-imports, next-match, override, package |
| `xsl:apply-imports` | 100+ | Import system ↑ | **1w** | Needs stylesheet precedence, symbol mangling |
| `xsl:next-match` | 80+ | Import + Template ↑ | **1w** | Needs template rule chaining |
| `xsl:override` | 50+ | Import system ↑ | **3d** | Override precedence |
| `xsl:package` / `xsl:use-package` | 30+ | Import system ↑ | **1w** | Module system, visibility control |
| Schema import/validation | <10 | XML Schema system | **2-3 weeks** | Deep integration |
| **Streaming** (accumulator, fork) | <5 | Streaming IR | **1-2 weeks** | Deprioritized |
| `xsl:evaluate` | 8+ | Dynamic expression eval | **2d** | Easier than import, medium complexity |
| **SUBTOTAL** | **1,283+** | | **5-7 weeks** | |

---

## Implementation Dependency Graph

```
┌─────────────────────────────────────────────────────────────────┐
│ PHASE 1: PARSE SCORE 1 ITEMS (26 hours)                        │
│ - Mode parsing (2h) → unblocks functions                        │
│ - Analyze-string (6h) → regexml ready                           │
│ - Character-map, message, assert, on-empty/non-empty (12h)      │
└──────────────────────────┬──────────────────────────────────────┘
                           ↓
┌─────────────────────────────────────────────────────────────────┐
│ PHASE 2: TEMPLATE & MODE SYSTEM (Score 2, 15 days)             │
│ - Mode parameter passing (4d)                                   │
│ - Template matching/priority (4d)                               │
│ - Call-template (3d)                                            │
│ - Pattern fixes (3d)                                            │
│ RESULT: ~500 tests → 1,800 total                               │
└──────────────────────────┬──────────────────────────────────────┘
                           ↓
┌─────────────────────────────────────────────────────────────────┐
│ PHASE 3: PARAMETERS & EARLY SCORE 2 (Score 2, 10 days)         │
│ - Parameters, variables (6d)                                    │
│ - Sorting (5d)                                                  │
│ RESULT: ~350 tests → 2,200 total                               │
└──────────────────────────┬──────────────────────────────────────┘
                           ↓
┌─────────────────────────────────────────────────────────────────┐
│ PHASE 4: OUTPUT & ATTRIBUTES (Score 2, 12 days)                │
│ - Output method, result-document (6d)                           │
│ - Attribute-set, copy/element fixes (6d)                        │
│ RESULT: ~200 tests → 2,400 total                               │
└──────────────────────────┬──────────────────────────────────────┘
                           ↓
┌─────────────────────────────────────────────────────────────────┐
│ PHASE 5: KEYS, GROUPING, ERROR HANDLING (Score 2, 15 days)    │
│ - Key support, for-each-group (6d)                              │
│ - Try/catch, iterate/break (6d)                                 │
│ - Remaining Score 2 items (3d)                                  │
│ RESULT: ~300 tests → 2,700 total                               │
└──────────────────────────┬──────────────────────────────────────┘
                           ↓
┌─────────────────────────────────────────────────────────────────┐
│ PHASE 6: IMPORT/INCLUDE SYSTEM ⭐ (Score 3, 15-20 days)        │
│ - Stylesheet composition architecture                           │
│ - Precedence/priority system                                    │
│ - Apply-imports, next-match support                             │
│ - Override, package support                                     │
│ RESULT: ~1,000 tests → 3,700+ total                            │
└──────────────────────────┬──────────────────────────────────────┘
                           ↓
┌─────────────────────────────────────────────────────────────────┐
│ PHASE 7: ADVANCED & EDGE CASES (Score 2-3, 10 days)            │
│ - Merge, evaluate, context items                                │
│ - Namespace-alias, number, decimal-format                       │
│ - Schema support (optional, complex)                            │
│ - Streaming (optional, deprioritized)                           │
│ RESULT: ~100+ tests → 3,800+ total                             │
└──────────────────────────┬──────────────────────────────────────┘
                           ↓
                    14,595 TESTS PASSING ✓
```

---

## Quick Implementation Checklist

### Priority 1 (Do in first 2 weeks)
- [ ] Parse `xsl:mode` (Score 1)
- [ ] Mode parameter passing (Score 2)
- [ ] Template matching/conflict resolution (Score 2)
- [ ] Call-template (Score 2)
- [ ] Pattern fixes (Score 2)

### Priority 2 (Weeks 3-4)
- [ ] Parameters & variables (Score 2)
- [ ] Sorting (Score 2)
- [ ] Analyze-string (Score 1)
- [ ] Output method (Score 2)
- [ ] Attribute-set (Score 2)

### Priority 3 (Weeks 5-7)
- [ ] Keys (Score 2)
- [ ] Error handling (Score 2)
- [ ] Iteration control (Score 2)
- [ ] For-each-group (Score 2)
- [ ] Message, assert, character-map (Score 1)

### Priority 4 (Weeks 8-10) ⭐ Big One
- [ ] **Import/Include system** (Score 3)
- [ ] Apply-imports (Score 3)
- [ ] Next-match (Score 3)
- [ ] Override (Score 3)

### Priority 5 (Optional, complex)
- [ ] Schema support (Score 3)
- [ ] Streaming (Score 3)
- [ ] Packages (Score 3)

---

## Test Analysis by Category

```
Category              | Pass | Fail | % | Key Blockers
──────────────────────┼──────┼──────┼───┼─────────────────────
Templates             |  4   |  2   | 67 | call-template
Functions             |  1   | 109  | 1% | mode declaration
Import                |  0   | 42   | 0% | import system
Apply-Templates       |  9   | 41   | 18%| patterns, modes
Type/Namespace        | ~200 | ~100 | 67%| mostly OK
Instructions          | ~100 | ~800 | 11%| various missing
Attributes            | ~100 | ~200 | 33%| attribute support
Sorting               |  0   | 120  | 0% | sort instruction
Variables/Params      |  0   | 200  | 0% | param system
Output                |  0   | 70   | 0% | output method
Keys                  |  0   | 60   | 0% | key support
────────────────────────────────────────────────────────────
TOTAL                 |1,098 |13,497|7.5%|
```

---

## Key Metrics for Progress Tracking

```
Starting Point:   1,098 / 14,595 (7.5%) ✗

After Quick Wins: 1,350 / 14,595 (9.3%)   [+252 tests]
After Phase 1-2:  2,000 / 14,595 (13.7%)  [+650 tests]
After Phase 1-3:  2,500 / 14,595 (17.1%)  [+1,150 tests]
After Phase 1-4:  2,700 / 14,595 (18.5%)  [+1,350 tests]
After Phase 1-5:  3,000 / 14,595 (20.6%)  [+1,650 tests]

Major Milestone:  4,000 / 14,595 (27%)    [After import planning]
Halfway Point:    7,297 / 14,595 (50%)    [After import impl starts]
Near Complete:   13,000 / 14,595 (89%)    [After Phase 6]
Goal:            14,595 / 14,595 (100%)   ✓
```

---

## Implementation Order Recommendation

**Start with these (Score 1, Quick Wins)**:
1. Mode parsing
2. Analyze-string (regexml available)
3. Character-map
4. Message/assert

**Then (Score 2, Core)**:
5. Mode parameter support
6. Template matching
7. Call-template
8. Parameters/variables
9. Sorting

**Then (Score 2, Supporting)**:
10. Output method
11. Attribute-set
12. Key support
13. Error handling

**Finally (Score 3, Architectural)**:
14. **Import/include** (biggest effort)
15. Optional: Schema, streaming

