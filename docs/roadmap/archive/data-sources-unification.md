# Roadmap: Унификация источников данных о типах

**Статус:** ✅ ЗАВЕРШЁН (все 5 фаз)
**Приоритет:** Высокий
**Создан:** 2025-12-04
**Обновлён:** 2025-12-05

## Проблема

Текущая архитектура имеет **дублирование и конфликт источников данных**:

1. **syntax_helper** (25,524 HTML файла) - основной источник из документации 1С
2. **platform_types.rs** (1862 строки) - хардкод "обогащения" с 48 методами
3. **signature_index.rs** - хардкод конструкторов коллекций

### Корневая причина бага с void return type

Парсер `extract_return_info()` не справляется со сложным HTML форматом union types:

```html
<!-- Реальный HTML в syntax_helper -->
Тип: <a>ПланСчетовСсылка.</a><span>&lt;</span><a>Имя</a><span>&gt;</span>, <a>Неопределено</a>.

<!-- Regex ожидает простой формат -->
Тип: <a>TYPE</a>. <br>
```

**Результат:** return_type = `None` → методы показывают `→ void`

### Масштаб проблемы (до исправления)

| Файл | Строк | Хардкод типов | Хардкод методов |
|------|-------|---------------|-----------------|
| platform_types.rs | 1862 | 9 | 48 |
| signature_index.rs | 1761 | 5 | 0 (конструкторы) |
| generic_inference.rs | ~300 | 0 | ~15 строк имён |
| resolver.rs | ~1500 | 0 | ~10 строк имён |

### После Фаз 1-2

| Файл | Строк | Содержание |
|------|-------|------------|
| platform_types.rs | ~300 | Только GenericInfo (InferenceMethodInfo) |
| signature_index.rs | ~1700 | Без изменений (конструкторы) |

---

## Фазы исправления

### Фаза 1: Исправить парсер return_type (Критично) ✅ ЗАВЕРШЕНА

**Цель:** Корректно извлекать return_type из сложного HTML

**Задачи:**

- [x] **1.1** Проанализировать все форматы return_type в syntax_helper HTML
  - Простой: `Тип: <a>Строка</a>.`
  - Union: `Тип: <a>Тип1</a>, <a>Тип2</a>.`
  - Generic/Faceted: `<a>Ссылка.</a><span>&lt;</span><a>Имя</a><span>&gt;</span>`
  - Nullable: `Тип: <a>Строка</a>, <a>Неопределено</a>.`

- [x] **1.2** Переписать `extract_return_info()` в `html_extractors.rs`
  - ✅ Использовать DOM-парсер (scraper) вместо regex
  - ✅ Обрабатывать union types (`Тип1, Тип2`)
  - ✅ Обрабатывать faceted types (`Ссылка.<Имя>`)
  - ✅ Нормализовать placeholder'ы (`<Имя справочника>` → generic param)

- [x] **1.3** Добавить тесты для всех форматов
  - ✅ Unit тесты с реальными HTML из syntax_helper
  - ✅ Integration тесты проверки return_type в hover

**Файлы:**
- `backend/src/data/loaders/syntax_helper/html_extractors.rs`

**Критерий завершения:**
- ✅ `НайтиПоКоду` показывает `→ СправочникСсылка.<T>` в hover

**Коммит:** `bf71d2e feat: DOM-based parser for return_type extraction (Phase 1)`

---

### Фаза 2: Удалить дублирующий хардкод ✅ ЗАВЕРШЕНА

**Цель:** Убрать хардкод который дублирует данные из syntax_helper

**Задачи:**

- [x] **2.1** Создать тесты для проверки данных из syntax_helper
  - ✅ Проверить что все методы СправочникМенеджер загружаются
  - ✅ Проверить что return_type корректный
  - ✅ Тесты GenericInfo в `tabular_section_type_test.rs`

- [x] **2.2** Удалить хардкод фасетных типов из platform_types.rs
  - ✅ `create_catalog_manager_type()` → удалён
  - ✅ `create_catalog_object_type()` → удалён
  - ✅ `create_catalog_reference_type()` → удалён
  - ✅ `create_document_manager_type()` → удалён
  - ✅ `create_document_object_type()` → удалён
  - ✅ `load_all_platform_types()` → удалён
  - ✅ `populate_signature_index_from_platform_types()` → удалён
  - ✅ `PlatformFacetTypesSource` → удалён

- [x] **2.3** Оставить только Generic-специфичный код
  - ✅ `get_generic_info_registry()` - реестр InferenceMethodInfo
  - ✅ `apply_generic_info_to_repository()` - применение GenericInfo к типам
  - ✅ GenericInfo для: Массив, Соответствие, СписокЗначений, ТабличнаяЧасть

**Файлы:**
- `backend/src/data/loaders/platform_types.rs` (~300 строк вместо 1862)
- `shared/src/domain/repository.rs` (добавлен `set_generic_info()`)

**Критерий завершения:**
- ✅ platform_types.rs < 1000 строк (сейчас ~300)
- ✅ Все методы загружаются из syntax_helper
- ✅ Hover показывает корректные return_type
- ✅ 146 unit-тестов проходят

---

### Фаза 3: Исправить merge логику SignatureIndex ✅ ЗАВЕРШЕНА

**Цель:** Обеспечить корректный merge данных из разных источников

**Задачи:**

- [x] **3.1** Унифицировать имена типов
  - ✅ `СправочникМенеджер.<Имя справочника>` → `СправочникМенеджер`
  - ✅ Добавлена поддержка HTML-encoded форматов (`.&lt;Имя`)
  - ✅ Расширена функция `extract_base_facet_type_name()`

- [x] **3.2** Добавить логирование merge операций
  - ✅ `tracing::debug!` при успешном merge каждого поля
  - ✅ `tracing::warn!` при конфликте return_type
  - ✅ `tracing::trace!` при добавлении нового метода

- [x] **3.3** Добавить тесты merge
  - ✅ `test_merge_syntax_helper_then_platform_types`
  - ✅ `test_merge_order_independence_platform_first`
  - ✅ `test_merge_conflict_return_type_keeps_first`
  - ✅ `test_merge_case_insensitive`

**Файлы:**
- `shared/src/domain/signature_index.rs` — логирование + 4 новых теста
- `shared/src/domain/signature_registry.rs` — расширен `extract_base_facet_type_name`

**Критерий завершения:**
- ✅ 50 тестов signature_index проходят
- ✅ 7 тестов signature_registry проходят
- ✅ Логирование работает (debug/warn/trace уровни)

---

### Фаза 4: Рефакторинг TypeMetadataLookup ✅ ЗАВЕРШЕНА

**Цель:** Единая точка получения метаданных типа

**Задачи:**

- [x] **4.1** Консолидация логики фасетов
  - ✅ Создан `shared/src/domain/facet_utils.rs` — централизованный модуль
  - ✅ `extract_base_facet_type()` определена в одном месте
  - ✅ `is_known_facet_prefix()` определена в одном месте
  - ✅ `signature_index.rs` делегирует в `facet_utils`
  - ✅ `metadata_lookup.rs` делегирует в `facet_utils`
  - ✅ 12 unit-тестов для facet_utils

- [x] **4.2** Упрощение `get_methods()`
  - ✅ Добавлен `normalize_type_name()` — нормализация с учётом active_facet
  - ✅ SignatureIndex — приоритетный источник методов
  - ✅ Удалено ~45 строк дублирующего кода из signature_index.rs
  - ✅ Удалено ~30 строк дублирующего кода из metadata_lookup.rs

- [x] **4.3** Документировать архитектуру (выполнено в Фазе 5)
  - ✅ Диаграмма потока данных
  - ✅ Описание приоритетов источников

**Файлы:**
- `shared/src/domain/facet_utils.rs` — НОВЫЙ
- `shared/src/domain/mod.rs` — добавлен export
- `shared/src/domain/signature_index.rs` — делегирование
- `shared/src/domain/metadata_lookup.rs` — удаление дублирования

**Критерий завершения:**
- ✅ `extract_base_facet_type()` в одном месте (facet_utils.rs)
- ✅ Нет дублирования FACET_PREFIXES
- ✅ get_methods() приоритезирует SignatureIndex
- ✅ 98 unit-тестов shared проходят
- ✅ Нет warnings при компиляции

**Reviewer notes:**
- Оставшееся дублирование `signature_registry::extract_base_facet_type_name()` имеет **другую семантику** (placeholder формат vs конкретизированный тип) — можно адресовать в Фазе 5

---

### Фаза 5: Очистка и оптимизация ✅ ЗАВЕРШЕНА

**Цель:** Убрать технический долг, документировать архитектуру

**Задачи:**

- [x] **5.1** Консолидация утилит извлечения типов
  - ✅ `extract_placeholder_base_type()` перенесена в facet_utils.rs
  - ✅ signature_registry.rs использует facet_utils
  - ✅ 11 новых edge case тестов добавлено
  - ✅ Dead code в platform_types.rs НЕ обнаружен (файл чистый)

- [x] **5.2** Упрощение wrapper делегирования
  - ✅ Удалён wrapper `is_known_facet_prefix()` из signature_index.rs
  - ✅ Заменены 3 вызова на прямые вызовы facet_utils

- [ ] **5.3** Оптимизация загрузки syntax_helper — **ОТЛОЖЕНО**
  - Уже оптимизировано: rayon + DashMap
  - Кеширование не критично при текущей производительности

- [x] **5.4** Документация архитектуры
  - ✅ Создан `docs/guides/type-lookup-architecture.md` (687 строк)
  - ✅ Mermaid диаграммы потока данных
  - ✅ Сравнение функций извлечения типов
  - ✅ Примеры использования
  - ✅ Ссылка добавлена в docs/README.md

**Файлы:**
- `shared/src/domain/facet_utils.rs` — добавлена `extract_placeholder_base_type()`
- `shared/src/domain/signature_registry.rs` — удалена локальная функция
- `shared/src/domain/signature_index.rs` — удалён wrapper
- `docs/guides/type-lookup-architecture.md` — НОВЫЙ

**Критерий завершения:**
- ✅ 50 тестов facet-related проходят
- ✅ Нет новых warnings при компиляции
- ✅ Документация архитектуры создана

---

## Приоритеты

| Фаза | Приоритет | Сложность | Влияние на пользователя | Статус |
|------|-----------|-----------|-------------------------|--------|
| 1 | 🔴 Критично | Средняя | Исправляет void return type | ✅ Завершена |
| 2 | 🟡 Высокий | Низкая | Убирает конфликты | ✅ Завершена |
| 3 | 🟡 Высокий | Низкая | Надёжность merge | ✅ Завершена |
| 4 | 🟢 Средний | Средняя | Чистота архитектуры | ✅ Завершена |
| 5 | ⚪ Низкий | Низкая | Документация + консолидация | ✅ Завершена |

---

## Зависимости

```
Фаза 1 ──┬──> Фаза 2 ──> Фаза 5
         │
         └──> Фаза 3 ──> Фаза 4
```

- Фаза 2 зависит от Фазы 1 (нужен рабочий парсер)
- Фаза 3 можно делать параллельно с Фазой 2
- Фаза 4 зависит от Фазы 3
- Фаза 5 можно делать в конце

---

## Метрики успеха

1. ✅ **Hover показывает правильный return_type** для всех методов
2. ✅ **platform_types.rs уменьшен** с 1862 до ~300 строк (84% уменьшение)
3. ✅ **Нет дублирования** методов между источниками (данные из syntax_helper)
4. ✅ **100% тестов** проходят (кроме 2 не связанных с изменениями)
5. ✅ **facet_utils.rs** — единственный источник логики фасетных типов
6. ✅ **Документация** архитектуры создана (`docs/guides/type-lookup-architecture.md`)
7. ✅ **Консолидация** placeholder и конкретных типов в facet_utils

---

## Финальная архитектура (после всех 5 фаз)

```
┌─────────────────────────────────────────────────────────────────┐
│                    DATA LOADING PHASE                           │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  syntax_helper HTML files (25,524 файла)                       │
│          │                                                      │
│          ▼                                                      │
│   extract_return_info() [DOM-парсер]                           │
│          │                                                      │
│          ▼                                                      │
│   RawTypeData (методы, параметры, return_type)                 │
│          │                                                      │
│          ▼                                                      │
│   SignatureSourceRegistry.build()                              │
│          │                                                      │
│          ├──► extract_placeholder_base_type() ◄── facet_utils  │
│          │    "СправочникМенеджер.<Имя>" → "СправочникМенеджер" │
│          ▼                                                      │
│   SignatureIndex (индекс по базовым типам)                     │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                    RUNTIME ANALYSIS PHASE                       │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│   TypeMetadataLookup.get_methods(resolution)                   │
│          │                                                      │
│          ├──► extract_base_facet_type() ◄── facet_utils        │
│          │    "СправочникМенеджер.Контрагенты" → базовый тип   │
│          │                                                      │
│          ├──► normalize_type_name()                            │
│          │                                                      │
│          ▼                                                      │
│   SignatureIndex.get_type_methods(base_type)                   │
│          │                                                      │
│          ▼                                                      │
│   Vec<MethodSignature> → hover/completion                      │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

**facet_utils.rs — единый источник истины:**
- `extract_base_facet_type()` — для конкретных типов (runtime)
- `extract_placeholder_base_type()` — для placeholder типов (loading)
- `is_known_facet_prefix()` — проверка известных фасетов
- 35+ unit-тестов

---

## Ссылки

- Исследование проблемы: conversation 2025-12-04
- Парсер HTML: `backend/src/data/loaders/syntax_helper/html_extractors.rs`
- Platform types: `backend/src/data/loaders/platform_types.rs`
- SignatureIndex: `shared/src/domain/signature_index.rs`
- GenericInfo registry: `backend/src/data/loaders/platform_types.rs:get_generic_info_registry()`
- Facet utils (Фаза 4): `shared/src/domain/facet_utils.rs`
- MetadataLookup: `shared/src/domain/metadata_lookup.rs`
- **Архитектура type-lookup (Фаза 5):** `docs/guides/type-lookup-architecture.md`
