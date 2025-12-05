# Roadmap: Унификация источников данных о типах

**Статус:** В работе (Фазы 1-3 завершены)
**Приоритет:** Высокий
**Создан:** 2025-12-04
**Обновлён:** 2025-12-04

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

### Фаза 4: Рефакторинг TypeMetadataLookup

**Цель:** Единая точка получения метаданных типа

**Задачи:**

- [ ] **4.1** Убрать дублирование логики поиска методов
  - `get_methods()` должен использовать только SignatureIndex
  - Убрать fallback на `find_type().methods`

- [ ] **4.2** Упростить `extract_type_name()`
  - Унифицировать обработку faceted types
  - Добавить кеширование результатов

- [ ] **4.3** Документировать архитектуру
  - Диаграмма потока данных
  - Описание приоритетов источников

**Файлы:**
- `shared/src/domain/metadata_lookup.rs`

**Критерий завершения:**
- Один источник правды для методов (SignatureIndex)
- Документация актуальна

---

### Фаза 5: Очистка и оптимизация

**Цель:** Убрать технический долг

**Задачи:**

- [ ] **5.1** Удалить неиспользуемый код
  - Найти dead code в platform_types.rs
  - Удалить устаревшие тесты

- [ ] **5.2** Оптимизировать загрузку syntax_helper
  - Lazy loading по требованию
  - Кеширование распарсенных данных

- [ ] **5.3** Обновить документацию
  - CLAUDE.md - убрать устаревшие инструкции
  - README - описать архитектуру данных

**Критерий завершения:**
- Нет warnings при компиляции
- Время загрузки типов < 5 сек

---

## Приоритеты

| Фаза | Приоритет | Сложность | Влияние на пользователя | Статус |
|------|-----------|-----------|-------------------------|--------|
| 1 | 🔴 Критично | Средняя | Исправляет void return type | ✅ Завершена |
| 2 | 🟡 Высокий | Низкая | Убирает конфликты | ✅ Завершена |
| 3 | 🟡 Высокий | Низкая | Надёжность merge | ✅ Завершена |
| 4 | 🟢 Средний | Средняя | Чистота архитектуры | ⏳ В очереди |
| 5 | ⚪ Низкий | Низкая | Производительность | ⏳ В очереди |

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
5. ⏳ **Время загрузки** типов приемлемое (< 10 сек) — не измерялось

---

## Текущая архитектура (после Фаз 1-2)

```
syntax_helper HTML files (25,524 файла)
        │
        ▼
 extract_return_info() [DOM-парсер]
        │
        ▼
 RawTypeData (методы, параметры, return_type)
        │
        ▼
 TypeRepository.load_types()
        │
        ▼
 apply_generic_info_to_repository() ◄── get_generic_info_registry()
        │                                   │
        ▼                                   └── InferenceMethodInfo
 TypeRepository с GenericInfo                   (Массив, Соответствие,
        │                                        СписокЗначений, ТабличнаяЧасть)
        ▼
 SignatureSourceRegistry.build()
        │
        ▼
 SignatureIndex (методы для hover/completion)
```

**Ключевые изменения:**
- Данные о методах приходят из syntax_helper (документация)
- GenericInfo (правила вывода типов) — допустимый хардкод (~100 строк)
- Удалены дублирующие определения типов

---

## Ссылки

- Исследование проблемы: conversation 2025-12-04
- Парсер HTML: `backend/src/data/loaders/syntax_helper/html_extractors.rs`
- Platform types: `backend/src/data/loaders/platform_types.rs`
- SignatureIndex: `shared/src/domain/signature_index.rs`
- GenericInfo registry: `backend/src/data/loaders/platform_types.rs:get_generic_info_registry()`
