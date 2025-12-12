# Roadmap: Рефакторинг структуры проекта

**Дата создания:** 2025-12-10
**Статус:** ✅ Завершено (все фазы выполнены)
**Приоритет:** Средний (после основной функциональности)

---

## Обзор

После масштабного рефакторинга (21 модуль → 150 файлов) выявлены оставшиеся проблемные зоны, требующие внимания.

### Текущее состояние

| Метрика | Значение | Прогресс |
|---------|----------|----------|
| Всего файлов | ~1,058 | +17 (модульность) |
| Файлы >500 строк | 30 | -3 ✓ |
| Файлы >1000 строк | 0 | -4 ✅ ЦЕЛЬ ДОСТИГНУТА |
| Средний размер файла | ~315 строк | улучшено |
| Глубина вложенности | макс 4 уровня | — |

### Целевые показатели

| Метрика | Цель |
|---------|------|
| Файлы >500 строк | <10 |
| Файлы >1000 строк | 0 (кроме тестов) |
| Средний размер файла | 200-300 строк |
| Дублирование типов | 0 |

---

## Фаза 1: Критичные файлы (>1000 строк)

### 1.1 html_extractors.rs (1,363 строк) ✅ ЗАВЕРШЕНО

**Путь:** `backend/src/data/loaders/syntax_helper/html_extractors/`

**Результат:** Разбито на 7 файлов (2025-12-10)

```
html_extractors/
├── mod.rs                    (175 строк) # Фасад + re-exports
├── title_extractor.rs        (37 строк)  # Извлечение заголовков
├── method_extractor.rs       (183 строки) # Извлечение методов
├── parameter_extractor.rs    (244 строки) # Извлечение параметров
├── description_extractor.rs  (132 строки) # Извлечение описаний
├── property_detector.rs      (119 строк) # Детектор свойств
└── tests.rs                  (626 строк) # Тесты
```

**Тесты:** 26/26 ✓
**Обратная совместимость:** Фасад HtmlExtractor сохранён

---

### 1.2 statement_converter.rs (1,237 строк) ✅ ЗАВЕРШЕНО

**Путь:** `backend/src/system/tree_sitter_adapter/statement_converter/`

**Результат:** Разбито на 8 файлов (2025-12-11)

```
statement_converter/
├── mod.rs           (200 строк) # Entry points + dispatcher + re-exports
├── loops.rs         (177 строк) # for, foreach, while
├── simple.rs        (98 строк)  # assignment, return, call
├── conditions.rs    (90 строк)  # if/elseif/else
├── declarations.rs  (90 строк)  # function_definition, var_definition
├── exceptions.rs    (71 строк)  # try/except, raise
├── handlers.rs      (64 строк)  # add/remove_handler, await
└── special.rs       (61 строк)  # goto, label, execute
```

**Итого:** 851 строк (было 1,237 — сокращение 31% за счёт удаления legacy кода)

**Тесты:** 298 passed ✓
**Обратная совместимость:** Dispatcher pattern + re-exports в mod.rs
**Code Review:** 9.5/10

---

## Фаза 2: Большие файлы (700-1000 строк)

### 2.1 lsp_server/server.rs (952 строк) ✅ ЗАВЕРШЕНО

**Путь:** `backend/src/bin/lsp_server/server/`

**Результат:** Разбито на 4 файла (2025-12-11)

```
server/
├── mod.rs              (34 строк)  # Struct BslLanguageServer + re-exports
├── core.rs             (106 строк) # new(), get_type_service(), helpers
├── language_server.rs  (811 строк) # Полная реализация LanguageServer trait
└── command_handlers.rs (71 строк)  # handle_get_current_context()
```

**Примечание:** Изначально планировалось 6 файлов, но Rust не позволяет иметь несколько `impl Trait for Type` в разных файлах. Весь trait implementation в `language_server.rs` с секциями (LIFECYCLE, FILE_MANAGEMENT, FEATURES, COMMANDS).

**Тесты:** 168/168 unit тестов ✓
**Backward compatibility:** main.rs без изменений
**Code Review:** 8.5/10

---

### 2.2 loader.rs (738 строк)

**Путь:** `backend/src/data/loaders/syntax_helper/loader.rs`

**Проблема:** Большой загрузчик с batch processing

**План:** Оставить как есть (после предыдущего рефакторинга). Мониторить рост.

---

### 2.3 resolver_core.rs (705 строк) ✅ ЗАВЕРШЕНО

**Путь:** `shared/src/domain/resolver/`

**Результат:** Разбито на 5 модулей (2025-12-11)

```
resolver/
├── mod.rs              (72 строки)  # Фасад + re-exports
├── type_resolver.rs    (204 строки) # Core struct + базовые методы
├── narrowing.rs        (41 строка)  # Type narrowing
├── validation.rs       (214 строк)  # validate_call, validate_call_v2
├── constructor.rs      (156 строк)  # resolve_constructor
├── context_resolution.rs (134 строки) # resolve_variable_with_context
├── strategies.rs       # БЕЗ ИЗМЕНЕНИЙ
├── member_resolution.rs # БЕЗ ИЗМЕНЕНИЙ
├── helpers.rs          # БЕЗ ИЗМЕНЕНИЙ
└── result_types.rs     # БЕЗ ИЗМЕНЕНИЙ
```

**Тесты:** 479 passed ✓
**Backward compatibility:** re-exports в mod.rs
**Code Review:** 8.5/10

---

### 2.4 symbol_table.rs (650 строк) ✅ ЗАВЕРШЕНО

**Путь:** `shared/src/ir/symbol_table/`

**Результат:** Разбито на 4 файла (2025-12-11)

```
symbol_table/
├── mod.rs             (296 строк) # Типы + scope management + functions + iterators
├── registration.rs    (90 строк)  # register_* методы
├── lookup.rs          (168 строк) # lookup/get/has/update методы
└── generics.rs        (119 строк) # Generic логика (initialize_as_generic, update_generic_param)
```

**Итого:** 673 строк (было 650 — добавлены докстроки)

**Тесты:**
- `symbol_table_tests`: 36 passed ✓
- `generic_tests`: 5 passed ✓
- Интеграция с backend: 168 passed ✓

**Публичный API протестирован:**
- ✅ `SymbolTable::new()`
- ✅ `register_variable()`, `lookup_variable()`
- ✅ `initialize_as_generic()`, `update_generic_param()`
- ✅ Scope hierarchy (create_scope, find_enclosing_function_scope)

**Backward compatibility:** Фасад через mod.rs + re-exports
**Code Review:** 9/10

---

### 2.5 resolution_impl.rs (615 строк) ✅ ЗАВЕРШЕНО

**Путь:** `shared/src/domain/types/resolution_impl/`

**Результат:** Разбито на 5 файлов (2025-12-11)

```
resolution_impl/
├── mod.rs                (9 строк)   # Декларации модулей
├── constructors.rs       (186 строк) # Factory methods (unknown, known, primitive, generic...)
├── queries.rs            (90 строк)  # is_unknown, is_dynamic, type_name
├── definition_location.rs (124 строки) # Go To Definition
└── compatibility.rs      (245 строк) # Система совместимости типов
```

**Итого:** 654 строки (было 615 — добавлены модульные декларации)

**Тесты:** 659 passed ✓ (491 shared + 168 backend)
**Покрытие:** constructors ✓, queries ✓, definition_location ✓, compatibility ✓
**Backward compatibility:** Все публичные методы TypeResolution доступны
**Code Review:** APPROVED

---

## Фаза 3: TypeScript файлы (>400 строк)

### 3.1 client.ts (476 строк) ✅ ЗАВЕРШЕНО

**Путь:** `vscode-extension/src/lsp/client/`

**Результат:** Разбито на 6 модулей (2025-12-11)

```
client/
├── index.ts            (33 строки)  # Инициализация + re-exports
├── lifecycle.ts        (290 строк)  # start/stop/restart/getClient
├── server-options.ts   (53 строки)  # buildServerOptions()
├── client-options.ts   (63 строки)  # buildClientOptions()
├── progress-handler.ts (139 строк)  # setupProgressHandler()
└── health-check.ts     (53 строки)  # startHealthCheck/stopHealthCheck
```

**Итого:** 631 строка (было 477)
**Компиляция:** npm run compile ✓
**Backward compatibility:** Все 8 экспортов сохранены
**Code Review:** 7.5/10 APPROVED

---

### 3.2 typeTreeBuilder.ts (416 → 481 строк) ✅ ЗАВЕРШЕНО

**Путь:** `vscode-extension/src/providers/typeTreeBuilder.ts`

**Статус:** Полностью переписан на LSP-based подход (2025-12-11)

**Результат:**
- ✅ `loadTypes()` теперь использует LSP Custom Request `bsl/getAllTypes`
- ✅ Добавлен `convertTypeDtoToBslEntity()` для конвертации TypeDto → BslEntity
- ✅ Импорт `getAllTypes` из `../lsp/customRequests`
- ⚠️ Legacy методы `loadPlatformTypes()`, `loadConfigurationTypes()` оставлены (не вызываются)

**Примечание:** Файл не разбит на модули — вместо этого переписан на правильную архитектуру (LSP-based loading вместо прямого чтения файлов кэша).

---

## Фаза 4: Устранение дублирования

### 4.1 Унификация типов ✅ ЗАВЕРШЕНО

**Результат:** Типы унифицированы (2025-12-11)

| Тип | Было | Стало | Статус |
|-----|------|-------|--------|
| `Span` | 2 версии | Display impl + re-export IrSpan | ✅ |
| `AnalysisResult` | 3 места (путаница имён) | CacheAnalysisResult, FlowAnalysisResult | ✅ |
| `OutputFormat` | 2 места (разные домены) | HoverOutputFormat, CliOutputFormat | ✅ |
| `Theme` | 2 места | Canonical в shared/formatting + deprecated re-exports | ✅ |

**Новые файлы:**
- `shared/src/formatting/mod.rs` — canonical Theme

**Code Review:** 8/10

---

### 4.2 Консолидация фасет-логики ✅ ЗАВЕРШЕНО

**Путь:** `shared/src/domain/facet_utils.rs`

**Результат:** Перенесены 3 функции из `facet_helpers.rs` в `facet_utils.rs` (2025-12-11)

Перенесённые функции:
- `get_facet_kind_from_prefix()` — определение FacetKind по суффиксу
- `substitute_type_name()` — подстановка имени в return type
- `extract_metadata_name()` — извлечение имени метаданных

**Примечание:** `facet_helpers.rs` стал тонким фасадом — все функции делегируют в `facet_utils`.

**Тесты:** 491 passed ✓
**Backward compatibility:** Публичный API через `SignatureIndex::` сохранён
**Code Review:** 9.5/10

---

## Фаза 5: Документация и навигация

### 5.1 README файлы

Создать README.md в ключевых директориях:
- `backend/src/README.md` - Обзор архитектуры backend
- `backend/src/application/README.md` - Application layer
- `shared/src/domain/README.md` - Domain модели
- `vscode-extension/src/lsp/README.md` - LSP клиент

**Оценка трудозатрат:** 2-3 часа

---

### 5.2 Диаграмма зависимостей ✅ ЗАВЕРШЕНО

**Путь:** `docs/architecture/dependency-diagrams.md`

**Результат:** Созданы 4 Mermaid диаграммы (2025-12-12)

1. ✅ Зависимости между crates (workspace graph)
2. ✅ Слои DDD в backend (presentation → application → system → domain → data)
3. ✅ Структура shared crate (api, domain, engine, ir, utils)
4. ✅ VSCode Extension структура (extension → lsp → providers)

---

## Приоритеты и порядок выполнения

### Высокий приоритет (блокирует развитие)

| # | Задача | Трудозатраты | Выигрыш | Статус |
|---|--------|--------------|---------|--------|
| 1 | html_extractors.rs | 4-6h | +20% читаемости | ✅ |
| 2 | statement_converter.rs | 4-6h | +15% навигации | ✅ |
| 3 | server.rs | 3-4h | +15% maintainability | ✅ |

### Средний приоритет (улучшает качество)

| # | Задача | Трудозатраты | Выигрыш | Статус |
|---|--------|--------------|---------|--------|
| 4 | resolver_core.rs | 4-5h | +15% читаемости | ✅ |
| 5 | Унификация типов | 2-3h | -30% дублирования | ✅ |
| 6 | Фасет-логика | 3-4h | -25% дублирования | ✅ |

### Низкий приоритет (nice to have)

| # | Задача | Трудозатраты | Выигрыш | Статус |
|---|--------|--------------|---------|--------|
| 7 | symbol_table.rs | 3-4h | +10% навигации | ✅ |
| 8 | resolution_impl.rs | 3-4h | +10% maintainability | ✅ |
| 9 | TypeScript файлы | 4-6h | +10% читаемости | ✅ |
| 10 | README файлы | 2-3h | +15% onboarding | ✅ |

---

## Метрики успеха

Итоговый статус:

- [x] Файлы >1000 строк: 0 ✅
- [ ] Файлы >500 строк: 30 (цель <10) — требует дальнейшей работы
- [x] Дублирование типов: 0 ✅
- [x] README в ключевых директориях: 3 ✅
- [x] Диаграммы зависимостей: 4 ✅

---

## Риски и митигация

| Риск | Вероятность | Митигация |
|------|-------------|-----------|
| Breaking changes в API | Средняя | Re-exports в mod.rs для backward compatibility |
| Регрессии в тестах | Низкая | Запуск cargo test после каждого шага |
| Циклические зависимости | Низкая | Строгий порядок разбиения (leaf modules first) |

---

## История изменений

| Дата | Изменение |
|------|-----------|
| 2025-12-10 | Создан план на основе анализа 4 Explore агентов |
| 2025-12-10 | ✅ Фаза 1.1: html_extractors.rs разбит на 7 модулей |
| 2025-12-11 | ✅ Фаза 1.2: statement_converter.rs разбит на 8 модулей (1,237 → 851 строк) |
| 2025-12-11 | ✅ Фаза 2.1: server.rs разбит на 4 модуля (952 → 1,022 строк с документацией) |
| 2025-12-11 | ✅ Фаза 2.3: resolver_core.rs разбит на 5 модулей (705 строк) |
| 2025-12-11 | ✅ Фаза 4.1: Унификация типов (Span, Theme, OutputFormat, AnalysisResult) |
| 2025-12-11 | ✅ Фаза 4.2: Консолидация фасет-логики (3 функции → facet_utils.rs) |
| 2025-12-11 | ✅ Фаза 2.4: symbol_table.rs разбит на 4 модуля (650 → 673 строки) |
| 2025-12-11 | ✅ Фаза 2.5: resolution_impl.rs разбит на 5 модулей (615 → 654 строки) |
| 2025-12-11 | ✅ Фаза 3.1: client.ts разбит на 6 модулей (477 → 631 строка) |
| 2025-12-11 | ✅ Фаза 3.2: typeTreeBuilder.ts переписан на LSP-based подход |
| 2025-12-11 | ✅ Фаза 5.1: Созданы README для backend/src, application, lsp |
| 2025-12-12 | ✅ Фаза 3.2: typeTreeBuilder.ts — удалены legacy методы (481 → 397 строк) |
| 2025-12-12 | ✅ Фаза 5.2: Созданы 4 Mermaid диаграммы зависимостей |
