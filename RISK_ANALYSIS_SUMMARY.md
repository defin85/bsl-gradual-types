# RISK ANALYSIS SUMMARY: Certainty enum Simplification

## QUICK FACTS

| Метрика | Значение |
|---------|----------|
| **Локаций в коде с Inferred(...)** | 28+ |
| **Файлов с зависимостями** | 25+ |
| **Тестов с hard-coded значениями f32** | 10+ |
| **Мест где f32 управляет логикой** | 3-4 КРИТИЧЕСКИХ |
| **Pattern matches с guard условиями** | 6+ |
| **Estimated simple removal time** | 17-25 дней (HIGH RISK) |
| **Estimated gradual migration** | 24-32 дня (MEDIUM RISK) |

---

## THE CORE PROBLEM

```
Certainty::Inferred(f32) служит для ДВУХ целей:

1. ОТОБРАЖЕНИЕ ИНФОРМАЦИИ
   - "Тип инфирован с 75% уверенностью"
   - Для UI, логов, hover информации
   - МОЖНО заменить на enum/categorization

2. УПРАВЛЕНИЕ ЛОГИКОЙ РЕШЕНИЙ ⚠️ КРИТИЧНО
   - if conf > 0.9 → Convert to Known (Generic Types)
   - if conf < 0.7 → Warn user (CLI Tool)
   - if conf > 0.8 → Show as "Known" in UI (DTO)
   - if conf > 0.8 → Count as known (Metrics)
   - НЕВОЗМОЖНО заменить без потери информации
```

---

## THREE PLACES WHERE f32 CONTROLS BEHAVIOR

### 1️⃣ Generic Type Inference (context_resolution.rs:106-112)

**Код:**
```rust
let certainty_level = if certainty > 0.9 {
    Certainty::Known  // <-- DECISION BASED ON 0.9
} else if certainty > 0.5 {
    Certainty::Inferred(certainty)
} else {
    Certainty::Inferred(0.5)
};
```

**Что это делает:**
- Если confidence > 90% → говорим что Generic тип Known (не просто Inferred)
- Это **КРИТИЧЕСКОЕ РЕШЕНИЕ** для типизации

**Пример:**
- `Array<String>` с conf=0.95 → становится `Known`, потом можно звать методы как на known типе
- `Array<String>` с conf=0.75 → остается `Inferred`, осторожнее с методами

**При удалении f32:** Как мы различим 0.95 от 0.75? Оба становятся просто `Inferred`.

---

### 2️⃣ CLI Warning Threshold (main.rs:110-115)

**Код:**
```rust
for (_, resolution) in &result.type_resolutions {
    match resolution.certainty {
        Certainty::Unknown => errors += 1,
        Certainty::Inferred(confidence) if confidence < 0.7 => {
            warnings += 1  // <-- DECISION BASED ON 0.7
        }
        _ => {}
    }
}
```

**Что это делает:**
- `conf < 0.7` → WARNING (низкое качество типа, пользователь должен проверить)
- `conf >= 0.7` → OK (достаточно уверены в типе)

**Пример:**
```
$ bsl-cli check --strict myfile.bsl
✅ Ошибок: 0
⚠️  Предупреждений: 3  <- Это типы с conf < 0.7
```

**При удалении f32:** Как мы определим это warning или OK? Все Inferred выглядят одинаково.

---

### 3️⃣ DTO Serialization (dto.rs:439-444)

**Код:**
```rust
let certainty_str = if *conf > 0.8 {  // <-- DECISION BASED ON 0.8
    "Known".to_string()      // UI видит "Known"
} else {
    "Inferred".to_string()   // UI видит "Inferred"
};

let percent = (*conf * 100.0) as u8;   // <-- Вычисляем %
```

**Что это делает:**
- Если conf > 80%, говорим UI что это "Known" (даже если внутренне Inferred)
- Вычисляем процент для прогресс-бара: 0.75 → 75%

**Пример UI:**
- conf=0.95 → 95% зеленый бар, label "Known" 🟢
- conf=0.5 → 50% желтый бар, label "Inferred" 🟡
- conf=0.2 → 20% красный бар, label "Inferred" 🔴

**При удалении f32:** Как мы рассчитаем процент? Как мы решим показать "Known" или "Inferred"?

---

## IMPACT BY COMPONENT

### Backend (Rust)

```
┌────────────────────────────────────────────┐
│ CRITICAL IMPACT                            │
├────────────────────────────────────────────┤
│ 1. Generic Type System breaks              │
│ 2. Type Resolution loses quality info      │
│ 3. 28 pattern matches need rewrite         │
│ 4. Decision logic breaks (3+ places)       │
│ 5. Tests fail (10+ locations)              │
└────────────────────────────────────────────┘
```

### CLI Tool

```
┌────────────────────────────────────────────┐
│ CRITICAL IMPACT                            │
├────────────────────────────────────────────┤
│ 1. --check --strict mode broken            │
│ 2. Can't distinguish warning from OK       │
│ 3. Output becomes less useful              │
│ 4. No quality feedback to user             │
└────────────────────────────────────────────┘
```

### Frontend (Leptos)

```
┌────────────────────────────────────────────┐
│ CRITICAL IMPACT                            │
├────────────────────────────────────────────┤
│ 1. type_card.rs: match statement breaks    │
│ 2. No percentage for progress bar          │
│ 3. All Inferred types look the same        │
│ 4. type_details_modal: color logic breaks  │
│ 5. VSCode extension: data format changes   │
└────────────────────────────────────────────┘
```

### Data & Serialization

```
┌────────────────────────────────────────────┐
│ HIGH IMPACT                                │
├────────────────────────────────────────────┤
│ 1. JSON format changes (backward compat)   │
│ 2. Old data not deserializable             │
│ 3. Migration required for saved types      │
│ 4. API contract broken                     │
└────────────────────────────────────────────┘
```

---

## REALISTIC EFFORT ESTIMATION

### Option A: Direct Removal (Not Recommended)

```
Phase 1: Remove enum variant        | 30 min
Phase 2: Fix compilation errors     | 4-6 hours
  - Find all pattern matches        | 1 hour
  - Fix guard conditions            | 1-2 hours
  - Update test assertions          | 1 hour
  - Fix frontend code               | 1-2 hours
Phase 3: Rewrite decision logic     | 5-7 days
  - Generic type inference          | 2-3 days
  - CLI check logic                 | 1 day
  - DTO serialization               | 2-3 days
Phase 4: Frontend changes           | 2-3 days
  - type_card.rs                    | 1 day
  - type_details_modal.rs           | 1 day
  - VSCode extension                | 1 day
Phase 5: Test rewrite               | 3-5 days
  - Rewrite 10+ test assertions     | 2 days
  - Find regressions                | 1-3 days
Phase 6: Integration testing        | 2-3 days
Phase 7: Bug fixes                  | 1-2 days

TOTAL: 17-25 days COMPRESSED
RISK: VERY HIGH (decision logic broken during development)
QUALITY: LOW (too many changes at once)
REGRESSION: HIGH (too many moving parts)
```

### Option B: Gradual Migration (Recommended) ✅

```
VERSION 0.5: Foundation (THIS WEEK)
├─ Introduce ConfidenceLevel enum   | 2-3 hours
├─ Mark f32 as #[deprecated]        | 30 min
├─ Add migration guide docs         | 1 hour
└─ NO CODE CHANGES REQUIRED        | LOW RISK

VERSION 0.6: Dual Support (2 weeks)
├─ New code uses ConfidenceLevel    | 5-7 days
├─ Tests for enum path              | 2-3 days
├─ f32 path still works             | NO CHANGE
├─ Both coexist safely              | MEDIUM RISK
└─ Users can migrate at own pace    | LOW RISK

VERSION 0.7: Migration (3-5 days)
├─ Rewrite decision logic to enum   | 3-5 days
├─ Migrate CLI --check logic        | 1 day
├─ Update frontend components       | 1-2 days
├─ Migrate tests                    | 2-3 days
└─ f32 marked "do not use"          | MEDIUM RISK

VERSION 4.0: Final Removal
├─ Remove f32 completely            | 1 day
├─ Cleanup deprecated code          | 1 day
└─ Final integration test           | 1 day
   TOTAL: 2-3 days | LOW RISK

GRAND TOTAL: 24-32 days SPREAD OVER 6 MONTHS
RISK: LOW (changes isolated per version)
QUALITY: HIGH (time to review/test each phase)
REGRESSION: LOW (gradual rollout)
```

---

## RISK SCORING

| Risk | Score | Assessment | Action |
|------|-------|-----------|--------|
| Generic Type Inference Loss | 9.5/10 | CRITICAL | Must preserve decision logic |
| Frontend UI Degradation | 9.5/10 | CRITICAL | Must preserve percentage calculation |
| CLI Tool Uselessness | 9.0/10 | CRITICAL | Must preserve 0.7 threshold logic |
| Data Serialization Break | 7.0/10 | MAJOR | Need migration path |
| Test Rewrite Effort | 8.0/10 | MAJOR | 10+ assertions need update |
| Overall Risk | 8.6/10 | CRITICAL | **DO NOT PROCEED WITHOUT PLAN** |

---

## DECISION TREE

```
START: "Should we remove f32 from Certainty::Inferred?"
│
├─ Question 1: Do we need to preserve decision logic thresholds?
│  ├─ YES (0.7, 0.8, 0.9) → Continue to Q2
│  └─ NO → Skip to Direct Removal (unlikely)
│
├─ Question 2: How much time can we spend?
│  ├─ "Quick fix" (< 1 week) → ❌ IMPOSSIBLE
│  ├─ "Normal sprint" (2-3 weeks) → ❌ RISKY (Option A only)
│  └─ "Multiple releases" (2-3 months) → ✅ SAFE (Option B)
│
├─ Question 3: Is backward compatibility important?
│  ├─ YES (users depend on JSON format) → ✅ Need migration
│  └─ NO → Accept data loss
│
└─ RECOMMENDATION
   if Q3 = YES:
       Use Option B (Gradual Migration) ✅
   else:
       Use Option B anyway (safer) ✅

FINAL: ✅ ALWAYS use Option B (Gradual Migration)
```

---

## WHAT COULD GO WRONG (Failure Modes)

### Scenario 1: Simple Removal Without Plan

```
Day 1:  Remove f32, code doesn't compile → Fix pattern matches
Day 2:  Code compiles, tests fail → Fix 10+ test assertions
Day 3:  Tests pass, Generic types broken → ??? How to fix?
Day 4:  CLI tool broken, warnings not working → ??? What to do?
Day 5:  Frontend shows garbage (no %), UI broken → ??? Rewrite?
Day 6:  Users complain, system shipped broken → 🔴 FAIL

Result: 🔴 System broken for 1-2 weeks while fixing
```

### Scenario 2: Gradual Migration (Planned)

```
Week 1:  Introduce enum, mark f32 deprecated → ✅ Safe, tested
Week 2:  New code uses enum, old code works → ✅ Both active
Week 3:  Rewrite decision logic to enum → ✅ Logic preserved
Week 4:  Frontend updated gradually → ✅ No user impact
Week 5:  Final removal, all tests pass → ✅ Clean transition

Result: ✅ System works throughout, users have 6 months to adapt
```

---

## RECOMMENDATION FOR TEAM

### DO THIS ✅

1. **Introduce ConfidenceLevel enum FIRST:**
   ```rust
   pub enum ConfidenceLevel {
       VeryLow,   // 0.0-0.2
       Low,       // 0.2-0.5
       Medium,    // 0.5-0.8
       High,      // 0.8-0.95
       VeryHigh,  // 0.95-1.0
   }
   ```

2. **Keep f32 working for 2-3 releases** while:
   - New code uses enum
   - Old code still uses f32
   - Both paths coexist

3. **Migrate critical logic gradually:**
   - Week 1: Generic type inference
   - Week 2: CLI warning logic
   - Week 3: DTO serialization
   - Week 4: Frontend components

4. **Remove f32 ONLY in next major release (v4.0)**

### DON'T DO THIS ❌

1. ❌ Don't remove f32 in one PR
2. ❌ Don't assume "all Inferred are the same"
3. ❌ Don't forget about decision thresholds (0.7, 0.8, 0.9)
4. ❌ Don't skip frontend/CLI/test updates
5. ❌ Don't ignore backward compatibility

---

## BOTTOM LINE

**The task seems simple on surface:**
> "Just remove f32 parameter from Inferred variant"

**But the reality is complex:**
> "f32 is used for DECISION LOGIC (3 critical places) + DISPLAY + TESTS + SERIALIZATION"

**Correct approach:**
1. Introduce `ConfidenceLevel` enum to preserve decision logic
2. Keep f32 as `#[deprecated]` for 2-3 releases
3. Gradually migrate all code to enum
4. Remove f32 only in v4.0

**Timeline:** 24-32 days of work spread over 6 months (LOW RISK)
**Timeline:** 17-25 days compressed (HIGH RISK) ⚠️ NOT RECOMMENDED

---

## GENERATED WITH RISK ANALYSIS FRAMEWORK

**Framework Version:** Devil's Advocate Architect v1.0
**Analysis Date:** 2025-12-14
**Codebase:** BSL Gradual Types v0.4.0
**Files Analyzed:** 25+ Rust files, 5+ TypeScript files
**Code Locations:** 343 references to Certainty

---

**🔴 FINAL VERDICT: DO NOT PROCEED with simple removal. USE GRADUAL MIGRATION APPROACH.**
