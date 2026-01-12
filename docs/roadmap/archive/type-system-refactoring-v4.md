# Type System Refactoring v4.0

**Дата создания:** 2025-12-13
**Обновлено:** 2025-12-14
**Статус:** ✅ ЗАВЕРШЁН
**Приоритет:** HIGH (понижен с CRITICAL после аудита)
**Затрачено времени:** ~1.25 часа (вместо оценочных ~3.5 недель)

## Контекст

Анализ архитектуры выявил проблемы в системе типов:
- ~~5 параллельных путей резолвинга~~ → **ОПРОВЕРГНУТО** (см. R-1)
- Распределённая фасетная логика в 6 файлах
- Неиспользуемые фасеты и f32 в Certainty
- 8-шаговая стратегия без документации порядка

---

## Milestone R-1: ~~CRITICAL~~ ЗАКРЫТ — Консолидация путей резолвинга

**Статус:** ✅ ЗАКРЫТ — НЕ ТРЕБУЕТСЯ
**Дата закрытия:** 2025-12-14
**Причина:** Аудит показал, что архитектура корректна

### Результаты аудита (2025-12-14)

Проведён детальный аудит кода с участием 3 архитекторов (pragmatic, innovative, risk).

#### Изначальная гипотеза (ОПРОВЕРГНУТА)

```
❌ ПРЕДПОЛАГАЛОСЬ: 5 параллельных путей резолвинга

1. TypeResolver.resolve_expression_sync()     [type_resolver.rs]
2. MemberResolver.resolve()                   [member_resolution.rs]
3. TypeMetadataLookup.get_methods/properties  [metadata_lookup/core.rs]
4. AstToIrConverter (собственная логика)      [ast_to_ir/*.rs]
5. Strategy pattern (Union/Intersection)      [strategies.rs]
```

#### Реальная архитектура (КОРРЕКТНА)

```
✅ РЕАЛЬНОСТЬ: Единый путь с делегацией

Type system facade
       │
       ▼
AnalysisEngine
       │
       ▼
TypeResolver.resolve_expression_sync()  ◄── ЕДИНЫЙ ENTRY POINT
       │
       ├── MemberResolver (делегат, вызывается ВНУТРИ)
       ├── UnionStrategy (делегат)
       ├── IntersectionStrategy (делегат)
       ├── GenericStrategy (делегат)
       ├── NullableStrategy (делегат)
       └── Primitives fallback

TypeResolver.resolve_variable_with_context()
       │
       └── ВЫЗЫВАЕТ resolve_expression_sync() внутри (строка 57)!
```

#### Проверенные "критические баги"

| Баг | Статус | Результат проверки |
|-----|--------|-------------------|
| `resolve_expression_sync` vs `resolve_variable_with_context` — разные пути | ❌ НЕ баг | `resolve_variable_with_context()` вызывает `resolve_expression_sync()` внутри |
| `active_facet: None` перед `get_methods()` | ❌ НЕ критично | 5-уровневый fallback защищает код |
| AstToIrConverter имеет собственную логику | ❌ НЕ баг | Делегирует в TypeResolver, `is_global_collection()` — оптимизация |

#### Выводы

1. **MemberResolver** — вызывается только из `TypeResolver.resolve_member_access()`, не параллельный путь
2. **Strategies** — вызываются только из TypeResolver, не параллельные пути
3. **TypeMetadataLookup** — использует TypeResolution как INPUT, не резолвит сам
4. **AstToIrConverter** — делегирует в TypeResolver через `resolver.resolve_expression_sync()`

**Архитектура соответствует паттерну Delegation. Рефакторинг не требуется.**

---

## Milestone R-2: ~~HIGH~~ ЗАКРЫТ — Консолидация фасетной логики

**Статус:** ✅ ЗАКРЫТ — НЕ ТРЕБУЕТСЯ
**Дата закрытия:** 2025-12-14
**Причина:** Аудит показал отсутствие дублирования

### Результаты аудита (2025-12-14)

Проведён детальный аудит с участием 3 архитекторов.

#### Изначальная гипотеза (ОПРОВЕРГНУТА)

```
❌ ПРЕДПОЛАГАЛОСЬ: 1,200+ строк дублирования в 6 файлах

1. facets.rs (87 строк)                    — определение FacetKind
2. facet_utils.rs (778 строк)              — утилиты
3. metadata_lookup/facets.rs (292 строк)   — КОПИЯ утилит!
4. signature_index/facet_helpers.rs (74)   — ЕЩЁ КОПИЯ!
5. type_resolver.rs                        — своя логика фасетов
6. ast_to_ir/expression_converter.rs       — СНОВА своя логика!
```

#### Реальная архитектура (КОРРЕКТНА)

```
✅ РЕАЛЬНОСТЬ: Single Source of Truth + слоистая архитектура

facet_utils.rs (778 строк)          ← ЕДИНСТВЕННЫЙ source of truth
    │                                  (~300 логики + 450 тестов)
    │
    ├─► types/facets.rs (87 строк)           ← Enum definition (не дублирование)
    │
    ├─► signature_index/facet_helpers.rs     ← Делегация, не копия!
    │   (74 строки)                            5 функций → facet_utils
    │
    └─► metadata_lookup/facets.rs            ← УНИКАЛЬНАЯ domain логика!
        (292 строки)                           get_facet_methods(), get_facet_properties()
                                               Использует facet_utils, не дублирует

type_resolver.rs                     ← НЕ содержит фасетную логику
expression_converter.rs              ← НЕ содержит фасетную логику
```

#### Детальная проверка facet_helpers.rs

| Функция | Делегирует в | Тип |
|---------|-------------|-----|
| `extract_base_facet_type()` | `facet_utils::` | Делегация |
| `get_facet_kind_from_prefix()` | `facet_utils::` | Делегация |
| `substitute_type_name()` | `facet_utils::` | Делегация |
| `extract_metadata_name()` | `facet_utils::` | Делегация |
| `get_metadata_kind_from_prefix()` | `MetadataPatternRegistry::` | Делегация |

**Вывод:** 100% делегация, 0% дублирования логики.

#### Проверка metadata_lookup/facets.rs

Содержит **уникальную domain логику**, не копию:
- `get_platform_facet_type()` — mapping (MetadataKind, FacetKind) → платформенный тип
- `get_facet_methods()` — lazy lookup методов через TypeRepository + SignatureIndex
- `get_facet_properties()` — lazy lookup свойств

**Эти функции НЕЛЬЗЯ перенести в facet_utils** — они зависят от TypeRepository и SignatureIndex (циклические импорты).

#### Выводы

1. **facet_utils.rs** — уже единственный source of truth
2. **facet_helpers.rs** — тонкий adapter layer (74 строки делегации), упрощение не оправдано (низкий ROI)
3. **metadata_lookup/facets.rs** — уникальная domain логика, НЕ дублирование
4. **type_resolver.rs** и **expression_converter.rs** — НЕ содержат фасетную логику

**Архитектура соответствует DDD (разделение слоёв). Консолидация не требуется и даже вредна.**

---

## Milestone R-3: ~~Упрощение~~ ЗАВЕРШЁН — Certainty без f32

**Статус:** ✅ ЗАВЕРШЁН
**Дата завершения:** 2025-12-14
**Фактическое время:** ~1 час

### Проблема (была)

```rust
pub enum Certainty {
    Known,
    Inferred(f32),  // ← f32 НИКОГДА не используется в логике!
    Unknown,
}
```

### Решение (реализовано)

```rust
pub enum Certainty {
    Known,        // 100% — из метаданных, аннотаций
    Inferred,     // 80% — уверенный вывод (бывший >= 0.7)
    InferredWeak, // 50% — слабый вывод (бывший < 0.7)
    Unknown,      // 0% — тип не определён
}
```

**Улучшение:** Вместо простого удаления f32 добавлен `InferredWeak` для дифференциации качества inference.

### Задачи

- [x] **R-3.1:** Изменить enum Certainty (удалить f32) ✅
- [x] **R-3.2:** Grep + замена `Inferred(x)` → `Inferred`/`InferredWeak` ✅
- [x] **R-3.3:** Удалить f32 из WeightedType — не требуется ✅
- [x] **R-3.4:** Обновить тесты ✅
- [x] **R-3.5:** Проверка компиляции ✅

### Критерии успеха

- [x] `cargo build` проходит ✅
- [x] Все 820+ тестов проходят ✅
- [x] Упрощённый pattern matching везде ✅

---

## Milestone R-4: ~~Удаление мёртвых фасетов~~ ЗАКРЫТ — НЕ ТРЕБУЕТСЯ

**Статус:** ✅ ЗАКРЫТ — НЕ ТРЕБУЕТСЯ
**Дата закрытия:** 2025-12-14
**Причина:** Аудит показал, что фасеты активно используются

### Результаты аудита (2025-12-14)

#### Изначальная гипотеза (ОПРОВЕРГНУТА)

```
❌ ПРЕДПОЛАГАЛОСЬ: 4 мёртвых фасета с 0 использований
- Metadata     — 0 использований
- Constructor  — дублирует Manager
- Collection   — дублирует List
- Singleton    — 0 использований
```

#### Реальное состояние (ФАСЕТЫ ИСПОЛЬЗУЮТСЯ)

| Фасет | Использований | Где используется |
|-------|---------------|------------------|
| **Collection** | 25+ | `facet_utils`, `member_resolution`, `context_resolution` — табличные части, регистры |
| **Metadata** | 5+ | `config_metadata_parser` — ExchangePlan и др. |
| **Constructor** | 5+ | `property_detector` — парсинг Syntax Helper |
| **Singleton** | 3+ | `config_metadata_parser` — CommonModule |

#### Критичные зависимости

```rust
// Collection влияет на логику shows_properties()!
pub fn shows_properties(&self) -> bool {
    matches!(self, FacetKind::Object | FacetKind::Reference | FacetKind::Collection)
}
```

**Удаление Collection сломает отображение свойств для табличных частей и регистров.**

#### Выводы

1. **Collection** — НЕЛЬЗЯ удалять, влияет на `shows_properties()`
2. **Metadata/Constructor/Singleton** — используются в парсерах и конфигурации
3. Гипотеза о "мёртвых фасетах" была основана на неполном анализе

**Рефакторинг не требуется. Все 9 фасетов имеют реальное применение.**

---

## Milestone R-5: ~~Рефакторинг~~ ЗАВЕРШЁН — TypeResolver группировка

**Статус:** ✅ ЗАВЕРШЁН
**Дата завершения:** 2025-12-14
**Фактическое время:** ~15 минут

### Проблема (была)

7 последовательных if'ов без документации порядка.

### Решение (реализовано)

```rust
/// ## Порядок резолвинга
///
/// 1. Direct lookup — прямой поиск в TypeRepository
/// 2. Member access — составные имена (Справочники.Контрагенты)
/// 3. Composite types — Union | Intersection | Generic | Nullable
/// 4. Primitives fallback — базовые типы
pub fn resolve_expression_sync(&self, expression: &str) -> TypeResolution {
    // 1. Direct lookup
    if let Some(raw_type) = self.repository.find_type(expression) { ... }

    // 2. Member access
    if let Some((base, member)) = MemberResolver::parse_member_access(expression) { ... }

    // 3. Composite types — Union | Intersection | Generic | Nullable
    if let Some(resolution) = self.try_composite_type(expression) {
        return resolution;
    }

    // 4. Primitives fallback
    self.try_resolve_primitive(expression)
        .map(TypeResolution::known)
        .unwrap_or_else(TypeResolution::unknown)
}

/// Попытка разрешить composite type (Union, Intersection, Generic, Nullable)
fn try_composite_type(&self, expression: &str) -> Option<TypeResolution> { ... }
```

### Задачи

- [x] **R-5.1:** Создать `try_composite_type()` метод ✅
- [x] **R-5.2:** Рефакторинг `resolve_expression_sync()` ✅
- [x] **R-5.3:** Добавить документацию порядка (module-level + method-level) ✅
- [x] **R-5.4:** Проверить тесты — 76+ resolver тестов проходят ✅

### Критерии успеха

- [x] 4 логические группы вместо 7 if'ов ✅
- [x] Документация порядка в коде ✅
- [x] Все тесты проходят ✅

---

## Timeline (обновлён 2025-12-14)

```
✅ R-1: ЗАКРЫТ (архитектура корректна, делегация работает)
✅ R-2: ЗАКРЫТ (нет дублирования, DDD слои правильные)
✅ R-3: ЗАВЕРШЁН (Certainty без f32, добавлен InferredWeak)
✅ R-4: ЗАКРЫТ (фасеты используются, удаление не требуется)
✅ R-5: ЗАВЕРШЁН (TypeResolver группировка + документация)

🎉 ВСЕ MILESTONES ЗАВЕРШЕНЫ!
```

## Суммарная оценка (финальная)

| Milestone | Время | Сложность | Риск до | Риск после | Статус |
|-----------|-------|-----------|---------|------------|--------|
| **R-1** | ~~7-10 дней~~ | — | ~~9/10~~ | — | ✅ ЗАКРЫТ |
| **R-2** | ~~3-5 дней~~ | — | ~~8/10~~ | — | ✅ ЗАКРЫТ |
| **R-3** | ~1 час | LOW | 7/10 | 1/10 | ✅ ЗАВЕРШЁН |
| **R-4** | ~~1.5 часа~~ | — | ~~2/10~~ | — | ✅ ЗАКРЫТ |
| **R-5** | ~15 мин | LOW | 7/10 | 2/10 | ✅ ЗАВЕРШЁН |
| **ИТОГО** | **~1.25 часа** | — | — | — | ✅ ГОТОВО |

**Экономия времени:** ~2.5 недели (R-1, R-2, R-4 не требовались после аудита)

## Ожидаемые результаты

Все цели достигнуты:
- ~~✅ Один путь резолвинга~~ → **УЖЕ ЕСТЬ** (подтверждено аудитом R-1)
- ~~✅ Одно место для фасетной логики~~ → **УЖЕ ЕСТЬ** (подтверждено аудитом R-2)
- ~~✅ Упрощённый Certainty (readability)~~ → **ГОТОВО** (R-3: `Inferred` + `InferredWeak`)
- ~~✅ 5 фасетов вместо 9~~ → **НЕ ТРЕБУЕТСЯ** (R-4: все 9 фасетов используются)
- ~~✅ Документированный порядок резолвинга~~ → **ГОТОВО** (R-5: `try_composite_type()`)

## Зависимости (финальные)

```
✅ R-1: ЗАКРЫТ
✅ R-2: ЗАКРЫТ
✅ R-3: ЗАВЕРШЁН
✅ R-4: ЗАКРЫТ
✅ R-5: ЗАВЕРШЁН

🎉 Type System Refactoring v4.0 ЗАВЕРШЁН!
```

**Итог:** Из 5 milestones — 2 реализованы (R-3, R-5), 3 закрыты после аудита (R-1, R-2, R-4).
