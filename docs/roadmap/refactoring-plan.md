# Roadmap: Рефакторинг структуры проекта

**Дата создания:** 2025-12-10
**Статус:** Планирование
**Приоритет:** Средний (после основной функциональности)

---

## Обзор

После масштабного рефакторинга (21 модуль → 150 файлов) выявлены оставшиеся проблемные зоны, требующие внимания.

### Текущее состояние

| Метрика | Значение |
|---------|----------|
| Всего файлов | ~1,041 |
| Файлы >500 строк | 33 |
| Файлы >1000 строк | 4 |
| Средний размер файла | ~351 строк |
| Глубина вложенности | макс 4 уровня |

### Целевые показатели

| Метрика | Цель |
|---------|------|
| Файлы >500 строк | <10 |
| Файлы >1000 строк | 0 (кроме тестов) |
| Средний размер файла | 200-300 строк |
| Дублирование типов | 0 |

---

## Фаза 1: Критичные файлы (>1000 строк)

### 1.1 html_extractors.rs (1,363 строк)

**Путь:** `backend/src/data/loaders/syntax_helper/html_extractors.rs`

**Проблема:** Монолитный парсер HTML документации платформы

**План разбиения:**
```
html_extractors/
├── mod.rs              # Re-exports
├── title_extractor.rs  # Извлечение заголовков
├── method_extractor.rs # Извлечение методов
├── parameter_extractor.rs # Извлечение параметров
├── description_extractor.rs # Извлечение описаний
└── tests.rs            # Тесты (существует)
```

**Оценка трудозатрат:** 4-6 часов

---

### 1.2 statement_converter.rs (1,237 строк)

**Путь:** `backend/src/system/tree_sitter_adapter/statement_converter.rs`

**Проблема:** Конвертер всех типов statements в одном файле

**План разбиения:**
```
statement_converter/
├── mod.rs                  # Основной convert_statement()
├── assignment.rs           # Присваивания
├── control_flow.rs         # if/for/while/try
├── declarations.rs         # Процедуры/функции
├── calls.rs                # Вызовы методов
└── special.rs              # goto/label/execute/raise
```

**Оценка трудозатрат:** 4-6 часов

---

## Фаза 2: Большие файлы (700-1000 строк)

### 2.1 lsp_server/server.rs (952 строк)

**Путь:** `backend/src/bin/lsp_server/server.rs`

**Проблема:** Весь LSP сервер + file manager + diagnostics в одном файле

**План разбиения:**
```
server/
├── mod.rs              # Backend struct + main loop
├── file_manager.rs     # Управление открытыми файлами
├── diagnostics.rs      # Публикация диагностик
├── progress.rs         # Индексирование (существует)
└── lifecycle.rs        # Start/stop/restart
```

**Оценка трудозатрат:** 3-4 часа

---

### 2.2 loader.rs (738 строк)

**Путь:** `backend/src/data/loaders/syntax_helper/loader.rs`

**Проблема:** Большой загрузчик с batch processing

**План:** Оставить как есть (после предыдущего рефакторинга). Мониторить рост.

---

### 2.3 resolver_core.rs (705 строк)

**Путь:** `shared/src/domain/resolver/resolver_core.rs`

**Проблема:** Монолитный TypeResolver с разной логикой

**План разбиения:**
```
resolver/
├── mod.rs              # TypeResolver struct + основной метод
├── resolver_core.rs    # Базовая логика (уменьшить)
├── union_resolver.rs   # Union types
├── generic_resolver.rs # Generic types
├── constructor.rs      # New() конструкторы
└── strategies.rs       # Существует
```

**Оценка трудозатрат:** 4-5 часов

---

### 2.4 symbol_table.rs (650 строк)

**Путь:** `shared/src/ir/symbol_table.rs`

**Проблема:** Таблица символов + все методы в одном файле

**План разбиения:**
```
symbol_table/
├── mod.rs          # SymbolTable struct
├── lookup.rs       # Методы поиска
├── scope.rs        # Scope management
└── builder.rs      # Построение таблицы
```

**Оценка трудозатрат:** 3-4 часа

---

### 2.5 resolution_impl.rs (615 строк)

**Путь:** `shared/src/domain/types/resolution_impl.rs`

**Проблема:** Мега-impl блок TypeResolution

**План:** Выделить в traits (FacetResolution, MetadataResolution)

**Оценка трудозатрат:** 3-4 часа

---

## Фаза 3: TypeScript файлы (>400 строк)

### 3.1 client.ts (476 строк)

**Путь:** `vscode-extension/src/lsp/client.ts`

**План разбиения:**
```
lsp/
├── client.ts           # Lifecycle, init (~150)
├── diagnostics.ts      # Диагностика (~120)
├── health-check.ts     # Health monitoring (~100)
└── error-handler.ts    # Обработка ошибок (~100)
```

**Оценка трудозатрат:** 2-3 часа

---

### 3.2 typeTreeBuilder.ts (416 строк)

**Путь:** `vscode-extension/src/providers/typeTreeBuilder.ts`

**План разбиения:**
```
providers/
├── typeTreeBuilder.ts  # Основной builder (~150)
├── typeLoader.ts       # Загрузка типов (~130)
├── typeFormatter.ts    # Существует
└── typeCategorizer.ts  # Категоризация (~130)
```

**Оценка трудозатрат:** 2-3 часа

---

## Фаза 4: Устранение дублирования

### 4.1 Унификация типов

| Тип | Дубликаты | Решение |
|-----|-----------|---------|
| `Span` | ir/span.rs, presentation/lsp/position.rs | Оставить в shared/ir, удалить дубликат |
| `AnalysisResult` | 3 места | Унифицировать в shared/api |
| `OutputFormat` | 2 места | Вынести в shared/formatting |
| `Theme` | 2 места | Вынести в shared/formatting |

**Оценка трудозатрат:** 2-3 часа

---

### 4.2 Консолидация фасет-логики

**Проблема:** Логика фасетов разбросана по 4 файлам:
- `domain/facet_utils.rs`
- `domain/metadata_lookup/facets.rs`
- `domain/signature_index/index.rs`
- `domain/types/metadata.rs`

**Решение:** Консолидировать в `domain/facet_utils.rs`, остальные делегируют

**Оценка трудозатрат:** 3-4 часа

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

### 5.2 Диаграмма зависимостей

Создать Mermaid диаграммы:
- Зависимости между crates (backend ← shared)
- Слои DDD в backend
- Модули type_system

**Оценка трудозатрат:** 1-2 часа

---

## Приоритеты и порядок выполнения

### Высокий приоритет (блокирует развитие)

| # | Задача | Трудозатраты | Выигрыш |
|---|--------|--------------|---------|
| 1 | html_extractors.rs | 4-6h | +20% читаемости |
| 2 | statement_converter.rs | 4-6h | +15% навигации |
| 3 | server.rs | 3-4h | +15% maintainability |

### Средний приоритет (улучшает качество)

| # | Задача | Трудозатраты | Выигрыш |
|---|--------|--------------|---------|
| 4 | resolver_core.rs | 4-5h | +15% читаемости |
| 5 | Унификация типов | 2-3h | -30% дублирования |
| 6 | Фасет-логика | 3-4h | -25% дублирования |

### Низкий приоритет (nice to have)

| # | Задача | Трудозатраты | Выигрыш |
|---|--------|--------------|---------|
| 7 | symbol_table.rs | 3-4h | +10% навигации |
| 8 | resolution_impl.rs | 3-4h | +10% maintainability |
| 9 | TypeScript файлы | 4-6h | +10% читаемости |
| 10 | README файлы | 2-3h | +15% onboarding |

---

## Метрики успеха

После завершения всех фаз:

- [ ] Файлы >1000 строк: 0 (кроме тестов)
- [ ] Файлы >500 строк: <10
- [ ] Дублирование типов: 0
- [ ] README в ключевых директориях: 5+
- [ ] Диаграммы зависимостей: 3+

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
