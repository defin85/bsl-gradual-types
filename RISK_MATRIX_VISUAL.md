# RISK MATRIX: Visual & Quantitative Analysis

## 1. RISK PROBABILITY × IMPACT MATRIX

```
              IMPACT
         ┌─────────────────────┐
    HIGH │  CRITICAL  CRITICAL │
         │     #1        #2     │
         │    #5       #3 #7    │
         │                      │
 MEDIUM  │   #6       #4        │
         │            #8        │
         │                      │
    LOW  │                      │
         │                      │
         └─────────────────────┘
           LOW      HIGH
         PROBABILITY


LEGEND:
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
#1 = Generic Type Inference Logic
#2 = CLI Warning Threshold (0.7)
#3 = DTO Serialization & UI
#4 = Metrics Calculation
#5 = Frontend Dependencies
#6 = Test Failures
#7 = JSON Compatibility
#8 = Edge Cases
```

## 2. RISK SEVERITY SCORECARD

| # | РИСК | PROB | IMPACT | SCORE | RATING | MITIGATION EFFORT |
|---|------|------|--------|-------|--------|-------------------|
| 1 | Generic Type Inference потеряет качество | 95% | CRITICAL | 9.5 | 🔴 CRITICAL | **5-7 дней** |
| 2 | CLI --check --strict threshold сломается | 90% | HIGH | 9.0 | 🔴 CRITICAL | **2-3 дня** |
| 3 | DTO serialization & UI breaks | 95% | HIGH | 9.5 | 🔴 CRITICAL | **3-4 дня** |
| 5 | Frontend type_card match falls apart | 100% | HIGH | 10.0 | 🔴 CRITICAL | **2-3 дня** |
| 4 | Metrics lose granularity | 80% | MEDIUM | 8.0 | 🟡 MAJOR | **1 день** |
| 6 | 10+ tests need rewrite | 100% | MEDIUM | 10.0 | 🟡 MAJOR | **2-3 дня** |
| 7 | JSON deserialization breaks | 70% | HIGH | 7.0 | 🟡 MAJOR | **1-2 дня** (migration) |
| 8 | Edge case: defaults unclear | 60% | MEDIUM | 6.0 | 🟠 MODERATE | **1 день** |

---

## 3. CODE LOCATIONS AFFECTED

### By Risk Category

```
┌─────────────────────────────────────────────────────────────┐
│ CRITICAL RISK LOCATIONS (필수 수정)                        │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│ 1️⃣  Generic Type Inference (DECISION LOGIC)                │
│     📁 shared/src/domain/resolver/context_resolution.rs     │
│     📍 Lines: 106-112 (if certainty > 0.9 → Known)         │
│     Impact: All Generic<T> types lose quality info          │
│                                                              │
│ 2️⃣  CLI Check Threshold (WARNING LOGIC)                    │
│     📁 cli/src/main.rs                                      │
│     📍 Lines: 110-115 (if confidence < 0.7 → warning)      │
│     Impact: Can't distinguish high/low quality types       │
│                                                              │
│ 3️⃣  DTO Serialization (UI PIPELINE)                        │
│     📁 shared/src/ir/dto.rs                                 │
│     📍 Lines: 429-448 (if conf > 0.8 → "Known" in UI)     │
│     Impact: Frontend loses quality information              │
│                                                              │
│ 4️⃣  Frontend Type Card (VISUALIZATION)                     │
│     📁 frontend/src/components/type_card.rs                 │
│     📍 Lines: 21-27 (match c > 0.8 / c > 0.5)             │
│     Impact: All inferred types look the same (loss of UX)  │
│                                                              │
│ 5️⃣  DTO Metrics Calculation (STATISTICS)                   │
│     📁 shared/src/ir/dto.rs                                 │
│     📍 Lines: 492-500 (if conf > 0.8 → count as known)    │
│     Impact: Metrics become meaningless                      │
│                                                              │
└─────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────┐
│ MAJOR IMPACT LOCATIONS (重要な修正)                         │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│ 📋 Pattern Matching (28+ locations)                         │
│    • Value creation: Inferred(0.5), Inferred(0.8), etc.    │
│    • Guard conditions: if c > 0.8, if c < 0.7, etc.       │
│    • Value extraction: (*c * 100.0) as u8                 │
│                                                              │
│    Files affected:                                          │
│    - shared/src/analysis/type_guards.rs                    │
│    - shared/src/domain/flow_analysis.rs                    │
│    - shared/src/domain/resolver/ (4 files)                 │
│    - shared/src/ir/symbol_table/generics.rs               │
│    - backend/src/application/ast_to_ir/                    │
│    - backend/src/helpers/hover_formatter/                  │
│                                                              │
│ 📋 Test Assertions (10+ locations)                          │
│    • Exact value checks: assert_eq!(t.certainty, Inferred(0.75))│
│    • Range checks: assert!(matches!(c, Inferred(c) if c > 0.8)) │
│    • Panic messages: panic!(\"Expected Inferred(Тип)\")    │
│                                                              │
│    Files affected:                                          │
│    - shared/tests/uncertainty_reason_tests.rs              │
│    - shared/src/domain/types/tests/ (3 files)             │
│    - backend/tests/ (5 files)                              │
│                                                              │
│ 📋 Serialization Format (JSON/serde)                        │
│    • serde derives on Certainty enum                        │
│    • Old format: {\"Inferred\": 0.75}                       │
│    • New format: \"Inferred\" (no value!)                   │
│    • Backward compatibility break                          │
│                                                              │
│    Files affected:                                          │
│    - shared/src/domain/types/certainty.rs                  │
│    - All code that serializes TypeResolution               │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

---

## 4. DEPENDENCY CHAIN ANALYSIS

```
Flow-sensitive Analysis
(calculates confidence: f32)
        │
        ▼
┌──────────────────────────────┐
│ resolve_generic_from_hint()  │
│ ❌ CRITICAL DEPENDENCY       │
│ Uses: if conf > 0.9 → Known  │
└──────────┬───────────────────┘
           │
           ├─→ TypeResolution.certainty
           │       │
           │       ▼
           │   ┌─────────────────────────┐
           │   │ type_resolution_to_dto()│
           │   │ ❌ CRITICAL DEPENDENCY   │
           │   │ Uses: if conf > 0.8    │
           │   │ Outputs: percent, name  │
           │   └────────┬────────────────┘
           │            │
           │            ├─→ TypeResolutionDto
           │            │       │
           │            │       ▼
           │            │   ┌──────────────────────────┐
           │            │   │ Frontend Components       │
           │            │   │ type_card.rs             │
           │            │   │ type_details_modal.rs    │
           │            │   │ ❌ CRITICAL DEPENDENCY    │
           │            │   │ Uses: percent for bar    │
           │            │   │ Uses: conf for color     │
           │            │   └──────────────────────────┘
           │            │
           │            └─→ JSON serialization
           │                (backward compat issue)
           │
           └─→ CLI Tool (check_command)
               ❌ CRITICAL DEPENDENCY
               Uses: if conf < 0.7 → warning
```

---

## 5. EFFORT ESTIMATION

### Option 1: Simple Removal (BAD IDEA)

```
Task                          | Effort  | Risk  | Quality
──────────────────────────────┼─────────┼───────┼─────────
1. Remove f32 from enum       | 30 min  | LOW   | GOOD
2. Fix compilation errors     | 4-6 hrs | HIGH  | BAD
   - Pattern matching         | 2-3 hrs |       |
   - Guard conditions         | 1-2 hrs |       |
   - Test assertions          | 1 hr    |       |
3. Rewrite decision logic      | 5-7 day | CRIT  | BAD
   - Generic type inference   | 2-3 day |       |
   - CLI threshold logic      | 1 day   |       |
   - DTO serialization        | 2-3 day |       |
4. Frontend changes           | 2-3 day | HIGH  | BAD
5. Test rewrite + debug       | 3-5 day | CRIT  | BAD
6. Migration for old JSON     | 1-2 day | MED   | MED
──────────────────────────────┼─────────┼───────┼─────────
TOTAL                         | 17-25 days | VERY HIGH | POOR
```

### Option 2: Gradual Migration (RECOMMENDED)

```
Task                          | Effort  | Risk  | Quality
──────────────────────────────┼─────────┼───────┼─────────
Phase 1 (v0.5): Setup
1. Create ConfidenceLevel enum| 2-3 hrs | LOW   | GOOD
2. Mark f32 as deprecated     | 30 min  | LOW   | GOOD
3. Add migration guide        | 1 hr    | LOW   | GOOD
                              | ────────────────│
Phase 1 Total                 | 4 hrs   | LOW   | GOOD

Phase 2 (v0.6): New Code
1. New code uses enum         | 5-7 day | MEDIUM| GOOD
2. Tests for enum             | 2-3 day | LOW   | GOOD
3. Old f32 code still works   | -       | LOW   | GOOD
                              | ────────────────│
Phase 2 Total                 | 10 day  | MEDIUM| GOOD

Phase 3 (v0.7): Migration
1. Rewrite decision logic     | 3-5 day | MEDIUM| GOOD
2. Migrate tests              | 2-3 day | MEDIUM| GOOD
3. Update frontend            | 1-2 day | MEDIUM| GOOD
                              | ────────────────│
Phase 3 Total                 | 8-12 day| MEDIUM| GOOD

Phase 4 (v4.0): Removal
1. Remove deprecated f32      | 1 day   | LOW   | GOOD
2. Final testing              | 1-2 day | LOW   | GOOD
                              | ────────────────│
Phase 4 Total                 | 2-3 day | LOW   | GOOD

──────────────────────────────┼─────────┼───────┼─────────
TOTAL                         | 24-32 day | LOW | EXCELLENT
                              | (spread over 6-8 months)
```

---

## 6. QUALITY METRICS IMPACT

### Current State (with f32)

```
┌─────────────────────────────────────────────────┐
│ Type Quality Scoring                            │
├─────────────────────────────────────────────────┤
│                                                  │
│ Array<String> with confidence 0.95 (Generic)   │
│ ✅ Can distinguish: Very High Quality           │
│    - Generic type inference: Works perfectly    │
│    - CLI --check: No warning                    │
│    - UI: Shows 95% green badge                  │
│    - Metrics: Counted as "known_type"          │
│                                                  │
│ Array<String> with confidence 0.25 (Generic)   │
│ ⚠️  Can distinguish: Low Quality               │
│    - Generic type inference: Marked as Inferred│
│    - CLI --check: Shows warning                │
│    - UI: Shows 25% red badge                   │
│    - Metrics: Counted as "inferred_type"      │
│                                                  │
└─────────────────────────────────────────────────┘
```

### After Removing f32 (without enum)

```
┌─────────────────────────────────────────────────┐
│ Type Quality Scoring (DEGRADED)                 │
├─────────────────────────────────────────────────┤
│                                                  │
│ Array<String> with confidence ??? (Generic)     │
│ ❌ CANNOT distinguish: Unknown Quality          │
│    - Generic type inference: ??? (broken)       │
│    - CLI --check: ??? (can't decide)           │
│    - UI: Shows ??? badge                       │
│    - Metrics: ??? count                        │
│                                                  │
│ Array<String> with confidence ??? (Generic)     │
│ ❌ CANNOT distinguish: Unknown Quality          │
│    - Generic type inference: ??? (broken)       │
│    - CLI --check: ??? (can't decide)           │
│    - UI: Shows ??? badge (same as above!)      │
│    - Metrics: ??? count                        │
│                                                  │
└─────────────────────────────────────────────────┘
```

### With New ConfidenceLevel Enum (Proposed)

```
┌─────────────────────────────────────────────────┐
│ Type Quality Scoring (PRESERVED)                │
├─────────────────────────────────────────────────┤
│                                                  │
│ Array<String> with VeryHigh (Generic)           │
│ ✅ Can distinguish: Very High Quality           │
│    - Generic type inference: Works (uses enum)  │
│    - CLI --check: No warning                    │
│    - UI: Shows VeryHigh green badge             │
│    - Metrics: Counted as "known_type"          │
│                                                  │
│ Array<String> with VeryLow (Generic)            │
│ ⚠️  Can distinguish: Low Quality               │
│    - Generic type inference: Inferred(VeryLow) │
│    - CLI --check: Shows warning                │
│    - UI: Shows VeryLow red badge               │
│    - Metrics: Counted as "inferred_type"      │
│                                                  │
│ ℹ️  PRECISION LOSS: [0.0-1.0] → {5 levels}    │
│    • Before: 1000 possible values (0.000-0.999)│
│    • After: 5 discrete levels                  │
│    • Impact: Acceptable for UI/CLI decisions   │
│                                                  │
└─────────────────────────────────────────────────┘
```

---

## 7. RISK HEATMAP: Timeline

```
RISK PROBABILITY ↓ vs TIME →

       NOW    WEEK 1   WEEK 2   WEEK 3   WEEK 4
       ────────────────────────────────────────

🔴    ████████████████████████████████████████
      Generic Type Inference (not started)

🔴    ████████████████████████████████████████
      CLI Warning Logic (not started)

🔴    ████████████████████████████████████████
      DTO UI Serialization (not started)

🟡    ████████████████████████████░░░░░░░░░░░░
      Pattern Matching (manageable if done early)

🟠    ████████████░░░░░░░░░░░░░░░░░░░░░░░░░░░
      Tests (can be deferred to end)


MITIGATION STRATEGY:
▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓  Introduce ConfidenceLevel enum (parallel)
      │
      └─→ This REDUCES risk of 🔴 items to 🟡
```

---

## 8. GO/NO-GO DECISION MATRIX

| Criterion | With f32 (Current) | Without f32 | With Enum | RECOMMENDATION |
|-----------|-------------------|------------|-----------|-----------------|
| **Decision Logic Support** | ✅ Excellent | ❌ Impossible | ✅ Good | Use Enum |
| **Implementation Time** | ✅ N/A | ❌ 17-25 days | ⚠️ 24-32 days | Use Enum |
| **Data Loss** | ✅ None | ❌ Critical | ✅ Minimal | Use Enum |
| **Backward Compat** | ✅ N/A | ❌ Broken | ⚠️ Migration needed | Use Enum + Migration |
| **Frontend UX** | ✅ Good | ❌ Degraded | ✅ Good | Use Enum |
| **Testing** | ✅ Simple | ❌ Broken | ✅ Good | Use Enum |
| **Code Quality** | ✅ Clear intent | ❌ Ambiguous | ✅ Clear intent | Use Enum |

**FINAL VERDICT:** 🛑 **DO NOT REMOVE f32 without introducing ConfidenceLevel enum first**

---

## 9. RECOMMENDATION TIMELINE

```
SCENARIO A: Simple Removal (NOT RECOMMENDED)
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
Result: 🔴 Code broken, features lost, 17+ days, high risk


SCENARIO B: Gradual Migration (RECOMMENDED) ✅
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

v0.5 (THIS WEEK)          v0.6 (NEXT 2 WEEKS)    v0.7 (WEEK 4-5)      v4.0 (NEXT RELEASE)
┌─────────────────┐      ┌─────────────────┐   ┌─────────────────┐   ┌─────────────────┐
│ Setup           │      │ New Code        │   │ Migration       │   │ Removal         │
│                 │      │                 │   │                 │   │                 │
│ • Create enum   │      │ • Write enum-   │   │ • Rewrite logic │   │ • Delete f32    │
│ • Mark as @dep  │      │   based code    │   │ • Migrate tests │   │ • Final test    │
│ • Add guide     │  ──→ │ • Keep f32      │   │ • Update UI     │ ──→ │ • Release v4.0  │
│                 │      │   working       │   │ • No f32 new    │   │                 │
│ 4 hours work    │      │ • Both coexist  │   │ • Deprecate f32 │   │ 2-3 days work   │
│ LOW RISK        │      │ 10 days work    │   │ 8-12 days work  │   │ LOW RISK        │
│                 │      │ MEDIUM RISK     │   │ MEDIUM RISK     │   │                 │
└─────────────────┘      └─────────────────┘   └─────────────────┘   └─────────────────┘

BENEFITS:
✅ Users can migrate gradually
✅ Less code churn per release
✅ Better testing at each stage
✅ Reduced risk of regressions
✅ Clear migration path
```

---

## CONCLUSION

**Status:** 🔴 **CRITICAL - DO NOT PROCEED WITH SIMPLE REMOVAL**

**Recommended Action:**
1. **Introduce ConfidenceLevel enum** in parallel with keeping f32
2. **Mark f32 as deprecated** for 2-3 releases
3. **Migrate critical code** to use enum
4. **Remove f32 completely** in next major release (v4.0)

**Estimated Total Effort:** 24-32 days (spread over 6 months) instead of 17-25 days compressed

**Risk Reduction:** From 🔴 CRITICAL to 🟡 MAJOR to ✅ ACCEPTABLE
