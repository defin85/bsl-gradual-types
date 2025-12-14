# RISK ANALYSIS: Упрощение Certainty enum (удаление f32 из Inferred)

## EXECUTIVE SUMMARY

Удаление f32 параметра из `Certainty::Inferred` — **ОЧЕНЬ РИСКОВАННАЯ** операция, которая проникает во все слои архитектуры и затрагивает критическую логику принятия решений. В коде есть **3 места, где confidence реально используется в алгоритмах** (не только отображение), плюс проблемы с сериализацией и массовые changes в паттерн-матчинге.

**Рекомендация:** Перед приступлением нужно очень осторожно спланировать **что будет вместо уровней confidence** — потому что текущая система использует значения типа 0.5, 0.6, 0.7, 0.8, 0.9 для управления поведением.

---

## CRITICAL RISKS (блокеры)

### 1. **Потеря информации о уровне уверенности в алгоритмах (HIGH IMPACT)**

**ПРОБЛЕМА:** В коде есть МИНИМУМ 3 места, где f32 confidence используется для **управления логикой**, не просто для отображения:

#### a) Generic Type Inference (context_resolution.rs:106-112)
```rust
let certainty_level = if certainty > 0.9 {
    Certainty::Known
} else if certainty > 0.5 {
    Certainty::Inferred(certainty)
} else {
    Certainty::Inferred(0.5)
};
```
**Что здесь происходит:**
- Значения 0.9 и 0.5 используются как **триггеры для ИЗМЕНЕНИЯ ПОВЕДЕНИЯ**
- `certainty > 0.9` → конвертируем в `Known` (известный тип)
- `0.5 < certainty <= 0.9` → сохраняем реальное значение
- `certainty <= 0.5` → заменяем на 0.5 (default)

**РИСК при удалении f32:**
- Где мы будем хранить информацию, что один Generic тип инфер был с confidence 0.8, а другой с 0.2?
- Как мы различим типы, которые должны быть `Known` от типов, которые должны быть `Inferred`?
- Это приведет к **потере информации и неправильному типированию Generic коллекций**

**ВЕРОЯТНОСТЬ:** ВЫСОКАЯ (100% - код явно полагается на числовые сравнения)
**ИМПАКТ:** КРИТИЧЕСКИЙ (Generic types станут неправильными)

---

#### b) Проверка качества инфиринга в CLI Tool (cli/src/main.rs:110-115)
```rust
for (_, resolution) in &result.type_resolutions {
    match resolution.certainty {
        Certainty::Unknown => errors += 1,
        Certainty::Inferred(confidence) if confidence < 0.7 => {
            warnings += 1
        }
        _ => {}
    }
}
```
**Что здесь происходит:**
- Threshold `0.7` используется для КЛАССИФИКАЦИИ warning'ов
- `confidence < 0.7` → warning (низкое качество типа)
- `confidence >= 0.7` → считаем типом OK

**РИСК при удалении f32:**
- Как мы определим, что инфир был низкого качества?
- Без числовых значений CLI tool not сможет отличить хорошие инферы от плохих
- **Потеряется информация о quality of inference**

**ВЕРОЯТНОСТЬ:** ВЫСОКАЯ
**ИМПАКТ:** ВЫСОКИЙ (CLI tool станет бесполезен для validation)

---

#### c) DTO Serialization для Hover (shared/src/ir/dto.rs:429-446)
```rust
let (category, certainty_str, certainty_percent) = match &resolution.certainty {
    Certainty::Known => ("Platform".to_string(), "Known".to_string(), 100u8),
    Certainty::Inferred(conf) => {
        let percent = (*conf * 100.0) as u8;
        (
            if matches!(resolution.result, ResolutionResult::Generic(_)) {
                "Generic".to_string()
            } else {
                "Inferred".to_string()
            },
            if *conf > 0.8 {
                "Known".to_string()  // <-- DECISION LOGIC!
            } else {
                "Inferred".to_string()
            },
            percent,
        )
    }
    Certainty::Unknown => return None,
};
```
**Что здесь происходит:**
- Threshold `0.8` используется для ПЕРЕКЛАССИФИКАЦИИ типа в DTO
- `conf > 0.8` → DTO говорит что это "Known" несмотря на то, что внутренне это Inferred
- `conf <= 0.8` → DTO правильно говорит "Inferred"

**РИСК при удалении f32:**
- Как мы определим какой процент показать юзеру в hover?
- Как мы примем решение о том, показать ли "Known" или "Inferred" в UI?
- Frontend зависит от `certainty_percent` для визуализации (см. ниже)

**ВЕРОЯТНОСТЬ:** ВЫСОКАЯ
**ИМПАКТ:** ВЫСОКИЙ (UI hover станет неинформативен)

---

### 2. **Обратная совместимость: JSON Serialization (MEDIUM-HIGH IMPACT)**

**ПРОБЛЕМА:** `Certainty` имеет `#[derive(Serialize, Deserialize)]`. Если мы изменим структуру enum, старые JSON файлы не будут десериализоваться.

**Сценарий:**
1. В production существуют saved TypeResolution как JSON (кэши, логи, etc.)
2. Мы меняем Certainty::Inferred(f32) на Certainty::Inferred
3. Десериализация падает с ошибкой

**Пример текущего формата:**
```json
{
  "certainty": { "Inferred": 0.75 }
}
```

**После удаления f32:**
```json
{
  "certainty": "Inferred"
}
```

**РИСК:**
- Если есть persistence слой (кэширование в БД, файлы), старые данные будут неполезны
- Потребуется migration скрипт для всех сохраненных данных

**ВЕРОЯТНОСТЬ:** СРЕДНЯЯ (зависит от того, есть ли persistence, но сгодится)
**ИМПАКТ:** ВЫСОКИЙ (data corruption/loss)

---

### 3. **Потеря метрик в Symbol Table Analysis (shared/src/ir/dto.rs:489-500)**

**ПРОБЛЕМА:** Подсчет Known vs Inferred vs Unknown типов в метриках использует числовой threshold:

```rust
match &var_state.resolution.certainty {
    Certainty::Known => known_types += 1,
    Certainty::Inferred(conf) => {
        if *conf > 0.8 {
            known_types += 1;  // <-- Рассчитываем как Known если conf > 0.8
        } else {
            inferred_types += 1;
        }
    }
    Certainty::Unknown => unknown_types += 1,
}
```

**РИСК:**
- Без conf значения, мы не сможем различить Good vs Bad inferences
- Метрики станут менее информативны
- **Потеряется понимание качества анализа**

**ВЕРОЯТНОСТЬ:** ВЫСОКАЯ
**ИМПАКТ:** СРЕДНИЙ (метрики становятся бесполезны)

---

## SIGNIFICANT RISKS (требуют обработки)

### 4. **Pattern Matching: 28+ мест с `Inferred(_)` (HIGH EFFORT)**

**ПРОБЛЕМА:** Во всех 28+ местах где пишется `Inferred(...)` нам нужно **решить что делать с хранимым значением**.

**Типы паттернов:**

#### a) Создание с hard-coded значениями (12 мест)
```rust
// Текущее
certainty: Certainty::Inferred(0.5),
certainty: Certainty::Inferred(0.6),
certainty: Certainty::Inferred(0.8),
// ...
```
**После удаления f32:**
```rust
// Что здесь? Просто Inferred?
certainty: Certainty::Inferred,
```
**ВОПРОС:** Все эти значения (0.5, 0.6, 0.8) одинаково важны — где мы их "потеряем"?

#### b) Pattern matching с guard условиями (10 мест)
```rust
match certainty {
    Certainty::Inferred(c) if c > 0.8 => { ... },
    Certainty::Inferred(c) if c < 0.5 => { ... },
    _ => { ... }
}
```
**После удаления f32:**
- Эти guard условия **НЕВОЗМОЖНО реализовать**
- Нужно переписать всю логику

#### c) Декструктурирование для вычислений (6 мест)
```rust
Certainty::Inferred(c) => {
    let percent = (*c * 100.0) as u8;  // Конвертируем в проценты
    // ...
}
```
**После удаления f32:**
- Откуда мы возьмем информацию о процентах?

**ВЕРОЯТНОСТЬ:** 100% (все это нужно переделать)
**ИМПАКТ:** ОЧЕНЬ ВЫСОКИЙ (28 изменений требуют обдумки)

---

### 5. **Frontend Dependencies (Leptos TypeScript) (MEDIUM-HIGH IMPACT)**

**ПРОБЛЕМА:** Frontend зависит от `certainty_percent` для отображения прогресс-бара.

**Файлы:**
- `frontend/src/components/type_card.rs:21` — match на certainty для выбора CSS класса
- `frontend/src/components/type_details_modal.rs:34` — выбор иконки/цвета в зависимости от certainty
- `frontend/src/vscode/type_details_simple.rs:191` — форматирование для VSCode
- `frontend/src/vscode/type_details_app.rs:62` — конвертация certainty в JSON для VSCode

**Текущий код (type_card.rs):**
```rust
let variant = match certainty {
    Certainty::Known => "success",
    Certainty::Inferred(c) if c > 0.8 => "success",  // Хороший инфер
    Certainty::Inferred(c) if c > 0.5 => "warning",  // Средний инфер
    Certainty::Inferred(_) => "danger",              // Плохой инфер
    Certainty::Unknown => "dark",
};
```

**РИСК при удалении f32:**
- Как мы различим "хороший инфер" от "плохого" в UI?
- Прогресс-бар не будет работать (нет процентов)
- Иконки будут неправильные

**ВЕРОЯТНОСТЬ:** 100% (frontend будет сломан)
**ИМПАКТ:** ВЫСОКИЙ (UI станет неинформативен)

---

### 6. **Tests: 10+ ассертов с явными значениями f32 (MEDIUM)**

**Проблема:** Множество тестов проверяют КОНКРЕТНЫЕ значения confidence:

**Файлы:**
- `shared/tests/uncertainty_reason_tests.rs:142` — `Certainty::Inferred(0.5)`
- `shared/src/domain/types/tests/type_resolution_constructors_tests.rs:108` — `assert_eq!(t.certainty, Certainty::Inferred(0.75))`
- `shared/src/ir/tests/generic_tests.rs:31` — `assert!(matches!(res.certainty, Certainty::Inferred(c) if (c - 0.0).abs() < 0.001))`
- И еще 7 мест...

**РИСК:**
- Все эти ассерты нужно переписать
- Если просто удалить проверку значения — потеряем тестирование качества инфиринга

**ВЕРОЯТНОСТЬ:** 100% (тесты упадут)
**ИМПАКТ:** СРЕДНИЙ (можно переписать, но долго)

---

### 7. **CLI Tool `--check --strict` mode станет бесполезен (MEDIUM)**

**Проблема:** Strict mode полагается на `confidence < 0.7` для warning:

```rust
// cli/src/main.rs:112
Certainty::Inferred(confidence) if confidence < 0.7 => {
    warnings += 1
}
```

**РИСК:**
- Без этой информации `--check --strict` не может определить качество типов
- CLI tool станет менее полезным

**ВЕРОЯТНОСТЬ:** ВЫСОКАЯ (код явно используется)
**ИМПАКТ:** СРЕДНИЙ (CLI ломается)

---

## EDGE CASES (требуют обработки)

- [ ] **Что если конфигурация не загружена?** Текущий код возвращает `Inferred(0.5)` как default. После удаления f32 — как обозначить это?

- [ ] **Generic коллекции с неизвестными параметрами?** Текущий код использует 0.0 для "completely unknown params". Как это представить без f32?

- [ ] **Floating point comparisons in guards.** Код использует `>= 0.8`, `< 0.5` etc. Как это перепишется?

- [ ] **Миграция сохраненных типов.** Если есть JSON файлы с `Certainty::Inferred(0.75)` — они не десериализуются.

- [ ] **Backward compatibility в API.** Web API возвращает `certainty_percent: u8`. Клиенты на это полагаются. Если мы сломаем это — клиенты упадут.

---

## TECHNICAL DEBT (если приступать)

### Неявные constraints, которые никому не ясны:

1. **Почему именно 0.5, 0.6, 0.7, 0.8, 0.9?** Нигде нет документации о семантике этих значений.

2. **Есть ли где-то специализированная логика для отрезков [0.0-0.5), [0.5-0.8), [0.8-1.0]?** Если есть — это скрытая зависимость.

3. **Почему в некоторых местах default 0.5, в других 0.8?** Нет последовательной политики.

4. **Есть ли научное обоснование для пороговых значений?** Если есть (из статьи Balyuk & Popova) — нужно это задокументировать.

---

## QUESTIONS (нужно уточнить)

1. **Что ДОЛЖНО быть вместо f32?**
   - Enum с уровнями? `enum ConfidenceLevel { VeryLow, Low, Medium, High, VeryHigh }`
   - Отдельное поле в TypeResolution? `certainty: Certainty, confidence: u8`
   - Что-то еще?

2. **Как мы обрабатываем 3 критических места, где confidence управляет логикой?**
   - Generic type inference (если conf > 0.9 → Known, иначе Inferred)
   - CLI warning threshold (если conf < 0.7 → warning)
   - DTO переклассификация (если conf > 0.8 → "Known" в UI)

3. **Есть ли persistence слой, который сохраняет TypeResolution?** Если да — нужна migration.

4. **Может, имеет смысл оставить f32 просто как комментарий/документацию, не используя его?** Это было бы более гладко чем удаление.

---

## RECOMMENDATIONS

### Для pragmatic подхода:
1. **НЕ удаляйте f32** — есть слишком много мест где он использует реальную логику
2. Вместо этого:
   - Отметьте f32 как `#[deprecated]` с примечанием что это будет удалено в v4.0
   - Создайте enum `ConfidenceLevel` для новых мест
   - Постепенно мигрируйте на новый enum
   - Вся логика с guard условиями переходит на новый enum
3. Эта стратегия даст time для миграции frontend/CLI/тестов

**TIMELINE:** 2-3 версии (рассчитано на эволюцию)

### Для innovative подхода:
1. **Полный рефакторинг:**
   - Создайте enum `ConfidenceLevel { Zero, Low(0-33%), Medium(33-66%), High(66-99%), Perfect }`
   - Или просто u8 от 0-100 (более компактно)
   - Переноси всю логику на новый тип
   - Старый f32 переходит в комментарии/logs
2. Перепишите все тесты в новой парадигме
3. Frontend должен работать с enum вместо процентов
4. CLI tool должен work с enum классификацией

**TIMELINE:** 1-2 недели работы (довольно интенсивно)

---

## SUMMARY TABLE

| Риск | Вероятность | Импакт | Статус | Зависит от |
|------|-------------|--------|--------|-----------|
| Generic Type Inference потеряет качество | HIGH | CRITICAL | 🔴 БЛОКЕР | Нет альтернативного способа хранить conf |
| CLI --check --strict сломается | HIGH | HIGH | 🔴 БЛОКЕР | Нет способа определить < 0.7 |
| DTO метрики потеряют качество | HIGH | MEDIUM | 🟡 ЗНАЧИМО | Нет способа рассчитать % в UI |
| Frontend UI сломается | HIGH | HIGH | 🔴 БЛОКЕР | Нет процентов для прогресс-бара |
| 28+ pattern matches нужно переписать | HIGH | MEDIUM | 🟡 ЗНАЧИМО | Много работы, но доабль |
| 10+ тестов упадут | HIGH | MEDIUM | 🟡 ЗНАЧИМО | Нужно переписать asserts |
| JSON deserialization сломается | MEDIUM | HIGH | 🟡 ЗНАЧИМО | Потребуется migration |
| Unknown перевелся в "default 0.5" семантика | MEDIUM | MEDIUM | 🟡 ЗНАЧИМО | Нужна миграционная стратегия |

---

## FINAL RECOMMENDATION

**🛑 РЕКОМЕНДУЮ НЕ УДАЛЯТЬ f32 полностью.**

Вместо этого:
1. Оставьте f32 как есть
2. Создайте новый enum `ConfidenceLevel` для future использования
3. Постепенно мигрируйте новый code на enum
4. Удалите f32 в следующем major release (v4.0)

**Причина:** Слишком много мест полагается на конкретные числовые значения для управления поведением. Полное удаление приведет к потере информации и сломает 3+ критических места в коде.
