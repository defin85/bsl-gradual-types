# RISK ANALYSIS: Code Examples & Impact Details

## 1. CRITICAL RISK #1: Generic Type Inference Logic

### Current Code (shared/src/domain/resolver/context_resolution.rs:106-132)

```rust
pub(crate) fn resolve_generic_from_hint(
    &self,
    base_type: &str,
    type_params: &[String],
    certainty: f32,  // <-- INPUT: 0.0 - 1.0
) -> TypeResolution {
    // ... построение concrete_params ...

    // DECISION LOGIC на основе числовых значений f32:
    let certainty_level = if certainty > 0.9 {
        // ЕСЛИ confidence > 90%, конвертируем в Known
        Certainty::Known
    } else if certainty > 0.5 {
        // ЕСЛИ 50% < confidence <= 90%, сохраняем как Inferred с реальным значением
        Certainty::Inferred(certainty)
    } else {
        // ЕСЛИ confidence <= 50%, заменяем на default 0.5
        Certainty::Inferred(0.5)
    };

    TypeResolution {
        result: ResolutionResult::Generic(generic_type),
        certainty: certainty_level,  // <-- OUTPUT
        source: ResolutionSource::Inferred,
        // ... metadata ...
    }
}
```

### What This Code Does

1. **Принимает confidence: f32** от flow-sensitive анализа (например, "я на 80% уверен что это Array<String>")
2. **Принимает решение на основе конкретных threshold'ов:**
   - `> 0.9` → "уверен достаточно, объявляю это Known"
   - `0.5-0.9` → "оставляю как Inferred с реальной уверенностью"
   - `<= 0.5` → "неуверен очень, заменяю на default 0.5"
3. **Возвращает TypeResolution с новым certainty_level**

### Impact of Removing f32

**SCENARIO 1: Наивное удаление (просто Certainty::Inferred без параметра)**

```rust
// ПОСЛЕ удаления f32:
let certainty_level = if certainty > 0.9 {
    // КУДА ДЕВАТЬ certainty > 0.9 ???
    // Не знаю что делать, просто возвращу Known
    Certainty::Known
} else {
    // ВСЕ остальные значения (0.0, 0.5, 0.75, 0.8, 0.95 ...)
    // Становятся одним Certainty::Inferred
    Certainty::Inferred  // <-- ПОТЕРЯ ИНФОРМАЦИИ!
};
```

**Проблема:** Мы потеряли информацию:
- `Generic<String>` с conf=0.95 → `Inferred`
- `Generic<String>` с conf=0.05 → `Inferred`
- **Неразличимо!** But they're completely different!

**SCENARIO 2: Введение новой структуры (enum)**

```rust
enum ConfidenceLevel {
    VeryLow,      // 0.0-0.33
    Low,          // 0.33-0.66
    Medium,       // 0.66-0.80
    High,         // 0.80-0.95
    VeryHigh,     // 0.95-1.0
}

pub enum Certainty {
    Known,
    Inferred(ConfidenceLevel),  // Вместо f32
    Unknown,
}
```

**Переписываем логику:**
```rust
let certainty_level = if certainty > 0.9 {
    Certainty::Known
} else if certainty > 0.5 {
    // Как мы определим ConfidenceLevel из certainty: f32???
    let level = if certainty > 0.8 {
        ConfidenceLevel::High
    } else if certainty > 0.66 {
        ConfidenceLevel::Medium
    } else {
        ConfidenceLevel::Low
    };
    Certainty::Inferred(level)
} else {
    Certainty::Inferred(ConfidenceLevel::VeryLow)
};
```

**Проблема:** Мы ПРОИЗВОЛЬНО разделили диапазон 0.5-0.9 на квартили. Что если реальная логика нуждается в более тонких различиях? (например, 0.75 нужно обращать как High, но 0.74 как Medium?)

---

## 2. CRITICAL RISK #2: CLI Warning Threshold

### Current Code (cli/src/main.rs:109-117)

```rust
async fn check_command(
    path: &str,
    _format: &CliOutputFormat,
    verbose: bool,
    _strict: bool,
) -> anyhow::Result<()> {
    let engine = create_analysis_engine().await?;
    let result = engine.analyze_file(path).await?;

    let mut errors = 0;
    let mut warnings = 0;

    for (_, resolution) in &result.type_resolutions {
        match resolution.certainty {
            Certainty::Unknown => errors += 1,
            // MAGIC NUMBER: 0.7 используется для классификации
            Certainty::Inferred(confidence) if confidence < 0.7 => {
                warnings += 1  // <-- Это warning, не ошибка
            }
            _ => {}  // Остальные Inferred >= 0.7 считаются OK
        }
    }

    // Выводим результаты...
    println!("   • Ошибок: {}", errors);
    println!("   • Предупреждений: {}", warnings);

    if errors > 0 {
        std::process::exit(1);  // Fail if errors
    }
}
```

### What This Code Does

1. **Классификирует типы в 3 категории:**
   - `Unknown` → ERROR (fail с exit code 1)
   - `Inferred(c) где c < 0.7` → WARNING (продолжаем с предупреждением)
   - `Inferred(c) где c >= 0.7` или `Known` → OK (без warning)

2. **Threshold 0.7 — это хард-кодированное УПРАВЛЯЮЩЕЕ значение**

### Impact of Removing f32

```rust
// ПОСЛЕ удаления f32:
Certainty::Inferred(_) => {
    // Мы потеряли информацию о confidence
    // Как мы определим это warning или OK???
    // Два варианта:

    // Вариант 1: Все Inferred считаем WARNING
    warnings += 1;  // <-- ПРОБЛЕМА: слишком много warnings!

    // Вариант 2: Все Inferred считаем OK
    // <-- ПРОБЛЕМА: пропускаем низкокачественные инферы!
}
```

**Real-world impact:**
- Разработчик пишет код с опечаткой в имени типа
- Система инфирует `СправочникXXX` (не существует) с confidence 0.5
- Current: CLI показывает WARNING и разработчик видит проблему
- After removal: Мы не можем различить confidence 0.5 от 0.95 → либо все warnings, либо ничего

---

## 3. CRITICAL RISK #3: DTO Serialization & UI Classification

### Current Code (shared/src/ir/dto.rs:429-448)

```rust
fn type_resolution_to_dto(&self, resolution: &TypeResolution) -> Option<TypeResolutionDto> {
    if matches!(resolution.certainty, Certainty::Unknown) {
        return None;  // Unknown types не сериализуются
    }

    let type_name = resolution.type_name();

    // DECISION LOGIC на основе conf > 0.8:
    let (category, certainty_str, certainty_percent) = match &resolution.certainty {
        Certainty::Known => (
            "Platform".to_string(),
            "Known".to_string(),
            100u8,
        ),
        Certainty::Inferred(conf) => {
            // Вычисляем процент (для прогресс-бара в UI)
            let percent = (*conf * 100.0) as u8;  // 0.75 → 75%

            (
                if matches!(resolution.result, ResolutionResult::Generic(_)) {
                    "Generic".to_string()
                } else {
                    "Inferred".to_string()
                },
                // DECISION: Если conf > 0.8, говорим что это "Known" в UI!
                if *conf > 0.8 {
                    "Known".to_string()      // <-- UI будет видеть "Known"
                } else {
                    "Inferred".to_string()   // <-- UI будет видеть "Inferred"
                },
                percent,  // <-- Передаем % для прогресс-бара
            )
        }
        Certainty::Unknown => return None,
    };

    Some(TypeResolutionDto {
        name: type_name,
        category,
        certainty: certainty_str,
        certainty_percent,  // <-- Frontend использует это для визуализации
        // ... остальные поля ...
    })
}
```

### What This Code Does

1. **Конвертирует внутреннее TypeResolution в DTO для передачи клиентам**
2. **Вычисляет `certainty_percent: u8` из f32** для показа юзеру (например, "75%")
3. **Переклассифицирует тип в UI:** если conf > 0.8, показываем как "Known" даже если внутренне это Inferred
4. **Передает эту информацию frontend'у**

### Impact on Frontend

#### frontend/src/components/type_card.rs:21
```rust
let variant = match certainty {
    Certainty::Known => "success",
    Certainty::Inferred(c) if c > 0.8 => "success",  // Green badge
    Certainty::Inferred(c) if c > 0.5 => "warning",  // Yellow badge
    Certainty::Inferred(_) => "danger",              // Red badge
    Certainty::Unknown => "dark",                     // Grey badge
};

// Выбирает CSS класс для визуализации:
// .success = 🟢 зеленый бейдж
// .warning = 🟡 желтый бейдж
// .danger = 🔴 красный бейдж
// .dark = ⚫ темный бейдж
```

**Impact:**
- `conf = 0.95` → 🟢 зеленый (user trusted)
- `conf = 0.75` → 🟡 желтый (user cautious)
- `conf = 0.25` → 🔴 красный (user doesn't trust)

After removing f32:
```rust
let variant = match certainty {
    Certainty::Known => "success",
    Certainty::Inferred => "???",  // ЧТО ЗДЕСЬ??? danger? warning? success?
    Certainty::Unknown => "dark",
};
```

**ВСЕ типы Inferred выглядят одинаково!** Пользователь не видит разницы между надежным (0.95) и ненадежным (0.25) типом.

---

## 4. EDGE CASE: Default Confidence Values

### Issue: Multiple "magic" default values

```rust
// Type Guard Analysis (shared/src/analysis/type_guards.rs:93)
certainty: Certainty::Inferred(0.8),  // <-- 0.8 для type guards

// Flow Analysis (shared/src/domain/flow_analysis.rs:163)
certainty: Certainty::Inferred(0.6),  // <-- 0.6 для flow analysis

// Member Resolution (shared/src/domain/resolver/member_resolution.rs:124)
Certainty::Inferred(0.5),  // <-- 0.5 для member resolution

// Context Resolution fallback (shared/src/domain/resolver/context_resolution.rs:112)
Certainty::Inferred(0.5)   // <-- 0.5 для unknown generic params

// Uncertainty Reason Tests (shared/tests/uncertainty_reason_tests.rs:140)
Certainty::Inferred(0.5),  // <-- 0.5 для config not loaded
```

### Questions

1. **Почему разные значения?**
   - Type guards = 0.8 (высокое качество?)
   - Flow analysis = 0.6 (среднее?)
   - Member resolution = 0.5 (低?)

2. **Есть ли семантическое различие?** Или это просто произвольные выборы?

3. **Какой смысл для user?** Если один тип получил 0.5 а другой 0.8 — что это значит?

**После удаления f32:** Все эти различия исчезают, и мы потеряем информацию о качестве инфиринга в разных контекстах.

---

## 5. PATTERN MATCHING FALLOUT

### Issue: 28+ мест с pattern matching на f32

#### a) Guard Conditions (6 мест)
```rust
// shared/src/ir/dto.rs:439
if *conf > 0.8 { ... }

// shared/src/domain/resolver/context_resolution.rs:107
if certainty > 0.9 { ... }

// cli/src/main.rs:112
if confidence < 0.7 { ... }

// frontend/src/components/type_card.rs:21
if c > 0.8 { ... }
if c > 0.5 { ... }

// ... и еще ~5 мест ...
```

**Перепишем на enum ConfidenceLevel:**
```rust
match conf_level {
    ConfidenceLevel::High | ConfidenceLevel::VeryHigh => { ... },
    ConfidenceLevel::Medium => { ... },
    ConfidenceLevel::Low | ConfidenceLevel::VeryLow => { ... },
}
```

**Проблема:** Мы ПОТЕРЯЛИ точность! Threshold 0.8 был точный, а теперь "High" охватывает весь диапазон [0.8-1.0].

#### b) Value Extraction (3 места)
```rust
// shared/src/ir/dto.rs:432
let percent = (*conf * 100.0) as u8;

// shared/src/ir/tests/generic_tests.rs:121
Certainty::Inferred(c) => assert!((*c - 0.5).abs() < 0.01)

// backend/tests/type_narrowing_integration_test.rs:41
certainty: bsl_shared::domain::types::Certainty::Inferred(0.7),
```

**После удаления f32:** Как мы рассчитаем процент? Как мы тестируем конкретные значения?

---

## 6. SERIALIZATION COMPATIBILITY ISSUE

### Current Serialization Format

```json
{
  "type_resolutions": [
    {
      "name": "ТЗ",
      "type": {
        "Concrete": {
          "type_name": "ТаблицаЗначений",
          "facet": "Object"
        }
      },
      "certainty": {
        "Inferred": 0.75    // <-- f32 VALUE
      },
      "source": "Inferred",
      "metadata": { ... }
    }
  ]
}
```

### After Removing f32

```json
{
  "type_resolutions": [
    {
      "name": "ТЗ",
      "type": { ... },
      "certainty": "Inferred",  // <-- NO VALUE!
      "source": "Inferred",
      "metadata": { ... }
    }
  ]
}
```

### Backward Compatibility Issue

**If client has old JSON with `"certainty": { "Inferred": 0.75 }`:**

```rust
// Десериализация упадет с ошибкой:
// Error: unknown variant `Inferred`, expected unit variant or struct variant with 0 fields
```

**Solution needs:**
1. Миграция всех сохраненных JSON файлов
2. Или поддержка обоих форматов при десериализации
3. Или версионирование API

---

## 7. TEST FAILURES EXAMPLES

### Test 1: Exact Value Assertion (shared/src/domain/types/tests/type_resolution_constructors_tests.rs)

```rust
#[test]
fn test_with_confidence_value() {
    let t = TypeResolution::with_confidence_value(0.75);  // <-- Создание
    assert_eq!(t.certainty, Certainty::Inferred(0.75));   // <-- Проверка ТОЧНОГО значения
}
```

**After removing f32:**
```rust
#[test]
fn test_with_confidence_value() {
    let t = TypeResolution::with_confidence_value(0.75);
    assert_eq!(t.certainty, Certainty::Inferred);  // <-- Потеряли проверку конкретного значения!
    // Как мы теперь проверяем что 0.75 был обработан корректно?
}
```

### Test 2: Range-based Assertion (shared/src/ir/tests/generic_tests.rs)

```rust
#[test]
fn test_generic_inference_confidence() {
    let res = infer_generic_type("Array<String>");
    // Проверяем что confidence был вычислен близко к 0.5
    assert!(matches!(res.certainty, Certainty::Inferred(c) if (c - 0.5).abs() < 0.01));
}
```

**After removing f32:**
```rust
#[test]
fn test_generic_inference_confidence() {
    let res = infer_generic_type("Array<String>");
    assert!(matches!(res.certainty, Certainty::Inferred));  // <-- Что мы проверяем?
    // Потеряли информацию о том что confidence должна быть ~0.5!
}
```

---

## SUMMARY: Data Flow Impact

```
┌─────────────────────────────────────────────────────────────┐
│ AST/Flow Analysis (вычисляет confidence: f32)             │
└────────────────────┬────────────────────────────────────────┘
                     │
                     ▼
┌─────────────────────────────────────────────────────────────┐
│ Generic Type Inference                                     │
│ if conf > 0.9 → Known else Inferred(conf)                 │
│ ❌ DELETE f32 → Потеря решений на основе threshold'ов    │
└────────────────────┬────────────────────────────────────────┘
                     │
                     ▼
┌─────────────────────────────────────────────────────────────┐
│ DTO Serialization                                          │
│ certainty_percent = (conf * 100.0) as u8                  │
│ ❌ DELETE f32 → Не можем рассчитать %                     │
└────────────────────┬────────────────────────────────────────┘
                     │
                     ▼
┌─────────────────────────────────────────────────────────────┐
│ Frontend UI                                                 │
│ match percent: 75% → 🟡 warning, 95% → 🟢 success        │
│ ❌ DELETE f32 → Все типы выглядят одинаково              │
└────────────────────┬────────────────────────────────────────┘
                     │
                     ▼
┌─────────────────────────────────────────────────────────────┐
│ CLI Tool                                                    │
│ if conf < 0.7 → warning                                   │
│ ❌ DELETE f32 → Не можем классифицировать warning'и      │
└─────────────────────────────────────────────────────────────┘
```

**Вывод:** f32 не просто используется для отображения — он управляет **логикой принятия решений** во многих местах архитектуры.
