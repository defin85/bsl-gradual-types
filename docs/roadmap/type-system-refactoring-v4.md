# Type System Refactoring v4.0

**Дата создания:** 2025-12-13
**Обновлено:** 2025-12-14
**Статус:** В РАБОТЕ
**Приоритет:** HIGH (понижен с CRITICAL после аудита)

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

TypeSystemService
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

## Milestone R-3: Упрощение Certainty

**Приоритет:** 🟡 MEDIUM
**Оценка:** 1.5 часа
**Риск текущего состояния:** 7/10

### Проблема

```rust
pub enum Certainty {
    Known,
    Inferred(f32),  // ← f32 НИКОГДА не используется в логике!
    Unknown,
}
```

**Факты:**
- `Inferred(0.5)` vs `Inferred(0.9)` — разницы в поведении НЕТ
- f32 не участвует в валидации
- Усложняет pattern matching

### Решение

```rust
pub enum Certainty {
    Known,      // 100% — тип из метаданных
    Inferred,   // Выведен из контекста (без вероятности)
    Unknown,    // Неизвестен
}
```

### Задачи

- [ ] **R-3.1:** Изменить enum Certainty (удалить f32)
- [ ] **R-3.2:** Grep + замена `Inferred(x)` → `Inferred`
- [ ] **R-3.3:** Удалить f32 из WeightedType (если есть)
- [ ] **R-3.4:** Обновить тесты
- [ ] **R-3.5:** Проверка компиляции

### Критерии успеха

- [ ] `cargo build` проходит
- [ ] Все тесты проходят
- [ ] Упрощённый pattern matching везде

---

## Milestone R-4: Удаление мёртвых фасетов

**Приоритет:** 🟡 MEDIUM
**Оценка:** 1.5 часа
**Риск текущего состояния:** 2/10 (низкий, но cleanup нужен)

### Проблема

```rust
pub enum FacetKind {
    Manager,      // ✅ Активно используется
    Object,       // ✅ Активно используется
    Reference,    // ✅ Активно используется
    Selection,    // ✅ Используется
    List,         // ✅ Используется
    Metadata,     // ❌ 0 использований
    Constructor,  // ❌ Дублирует Manager
    Collection,   // ❌ Дублирует List
    Singleton,    // ❌ 0 использований
}
```

### Решение

```rust
pub enum FacetKind {
    Manager,    // Создание, поиск (Справочники.X)
    Object,     // Изменяемый объект (СправочникОбъект.X)
    Reference,  // Ссылка, read-only (СправочникСсылка.X)
    Selection,  // Выборка (СправочникВыборка.X)
    List,       // Список в форме (СправочникСписок.X)
}
```

### Задачи

- [ ] **R-4.1:** Grep использования Metadata, Constructor, Collection, Singleton
- [ ] **R-4.2:** Удалить из enum
- [ ] **R-4.3:** Обновить `shows_properties()`, `platform_suffix()`
- [ ] **R-4.4:** Обновить тесты
- [ ] **R-4.5:** Проверка компиляции

### Критерии успеха

- [ ] 5 фасетов вместо 9
- [ ] Все тесты проходят
- [ ] -30% complexity в фасетной системе

---

## Milestone R-5: Рефакторинг 8-шаговой стратегии TypeResolver

**Приоритет:** 🟢 LOW
**Оценка:** 35 минут
**Риск текущего состояния:** 7/10 (порядок критичен, не документирован)

### Проблема

```rust
// 8 последовательных if'ов, порядок критичен!
pub fn resolve_expression_sync(&self, expression: &str) -> TypeResolution {
    // 1. Direct lookup
    if let Some(raw_type) = self.repository.find_type(expression) { ... }
    // 2. Member access
    if let Some((base, member)) = ... { ... }
    // 3. Union
    if expression.contains('|') { ... }
    // 4. Intersection
    if expression.contains('&') { ... }
    // 5. Generic
    if expression.contains('<') && expression.contains('>') { ... }
    // 6. Nullable
    if expression.ends_with('?') { ... }
    // 7. Primitives
    // 8. Fallback
}
```

### Решение: Группировка composite types

```rust
pub fn resolve_expression_sync(&self, expression: &str) -> TypeResolution {
    // 1. Direct lookup
    if let Some(res) = self.try_direct_lookup(expression) {
        return res;
    }

    // 2. Member access (Base.Member)
    if let Some(res) = self.try_member_access(expression) {
        return res;
    }

    // 3. Composite types (Union | Intersection | Generic | Nullable)
    if let Some(res) = self.try_composite_type(expression) {
        return res;
    }

    // 4. Primitives fallback
    self.try_primitive(expression).unwrap_or_else(TypeResolution::unknown)
}

fn try_composite_type(&self, expr: &str) -> Option<TypeResolution> {
    if expr.contains('|') { return Some(self.resolve_union(expr)); }
    if expr.contains('&') { return Some(self.resolve_intersection(expr)); }
    if expr.contains('<') && expr.contains('>') { return Some(self.resolve_generic(expr)); }
    if expr.ends_with('?') { return Some(self.resolve_nullable(expr)); }
    None
}
```

### Задачи

- [ ] **R-5.1:** Создать `try_composite_type()` метод
- [ ] **R-5.2:** Рефакторинг `resolve_expression_sync()`
- [ ] **R-5.3:** Добавить документацию порядка
- [ ] **R-5.4:** Проверить тесты

### Критерии успеха

- [ ] 4 логические группы вместо 8 if'ов
- [ ] Документация порядка в коде
- [ ] Все тесты проходят

---

## Timeline (обновлён 2025-12-14)

```
✅ R-1: ЗАКРЫТ (архитектура корректна, делегация работает)
✅ R-2: ЗАКРЫТ (нет дублирования, DDD слои правильные)

Осталось: R-3, R-4, R-5 (Quick wins) — ~3.5 часа
├── R-3: Certainty без f32 (1.5 часа)
├── R-4: Мёртвые фасеты (1.5 часа)
└── R-5: TypeResolver группировка (35 минут)
```

## Суммарная оценка (обновлена)

| Milestone | Время | Сложность | Риск до | Риск после | Статус |
|-----------|-------|-----------|---------|------------|--------|
| **R-1** | ~~7-10 дней~~ | — | ~~9/10~~ | — | ✅ ЗАКРЫТ |
| **R-2** | ~~3-5 дней~~ | — | ~~8/10~~ | — | ✅ ЗАКРЫТ |
| **R-3** | 1.5 часа | LOW | 7/10 | 1/10 | ⏳ Quick win |
| **R-4** | 1.5 часа | LOW | 2/10 | 1/10 | ⏳ Quick win |
| **R-5** | 35 мин | LOW | 7/10 | 2/10 | ⏳ Quick win |
| **ИТОГО** | **~3.5 часа** | — | — | — | — |

**Экономия времени:** ~2.5 недели (R-1 + R-2 не требуются)

## Ожидаемые результаты

После завершения:
- ~~✅ Один путь резолвинга~~ → **УЖЕ ЕСТЬ** (подтверждено аудитом R-1)
- ~~✅ Одно место для фасетной логики~~ → **УЖЕ ЕСТЬ** (подтверждено аудитом R-2)
- ✅ Упрощённый Certainty (readability) — R-3
- ✅ 5 фасетов вместо 9 (simplicity) — R-4
- ✅ Документированный порядок резолвинга — R-5

## Зависимости (обновлены)

```
✅ R-1: ЗАКРЫТ
✅ R-2: ЗАКРЫТ

R-3 (Certainty) ◄── независим
R-4 (мёртвые фасеты) ◄── независим
R-5 (TypeResolver) ◄── независим

Все три можно делать параллельно или в любом порядке.
```

**Приоритет:** R-3 или R-5 (наибольшая польза). R-4 — cleanup.
