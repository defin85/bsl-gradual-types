# Архитектура: `bsl-agent` (MCP, локальная семантика проекта)

**Статус:** 🔴 ПЛАН  
**Приоритет:** HIGH  
**Референс:** `mcp-debug-server/` (паттерн `rmcp` + stdio + manager + resources)

---

## 1) Резюме решений (зафиксировано)

### 1.1. Как MCP получает семантику

**Решение:** MCP-сервер даёт семантику **in-proc** через общий `SemanticFacade`/`SemanticProvider`, а не через проксирование LSP.

**Почему не “общаться с уже запущенным LSP”:**

- текущий LSP сервер в проекте запускается как STDIO-only (`backend/src/bin/lsp_server/main.rs`) и не слушает TCP → подключиться вторым клиентом нельзя;
- даже при добавлении TCP, LSP почти всегда предполагает `1 server <-> 1 client` и хранит состояние документов (включая unsaved тексты) внутри сессии клиента;
- прокси через LSP добавляет отдельный процесс, сложную синхронизацию `didOpen/didChange`, async нотификации diagnostics и хуже контроль детерминизма/кэшей.

### 1.2. “Проектный” контекст для LLM

**Решение:** основной интерфейс для LLM — агрегирующий tool `context.pack`, который:

- принимает “цель” (goal) и “фокус” (diagnostic/symbol/file+pos/query);
- возвращает компактный текстовый пакет + структурированный список items;
- работает **в рамках бюджета** (chars/tokens) и умеет честно отвечать `completeness=partial` + `missing_inputs[]` при нехватке данных.

Точечные tools (`diagnostics`, `typeAtPosition`, `definition`, `references`, `members`, `symbol.search`) нужны только для дозапросов.

### 1.3. Local-first (на данном этапе)

**Решение:** первая версия — **локальная**:

- MCP читает FS workspace напрямую (roots sandbox).
- Семантика строится in-proc, без “тонкого агента” к remote Semantic Server.

**Заложено на будущее:** интерфейс `SemanticProvider` позволяет добавить remote режим позже (без смены MCP API).

### 1.4. Unsaved buffer из IDE

**Решение:** поддержать **опционально**:

- если клиент передаёт `text`, анализируем его как snapshot файла;
- если `text` не передан, читаем с диска.

Это критично для IDE-host’ов (VS Code/Cursor), но не мешает CLI-host’ам.

---

## 2) Цели и нефункциональные требования

### 2.1. Functional

- `diagnostics` по файлу и по проекту (с группировками).
- “семантическая навигация”:
  - `typeAtPosition`
  - `members` (member list / completion-like для receiver выражений)
  - `definition`
  - `references`
  - `symbol.search`
- `context.pack`: собрать контекст “готовый для LLM” под задачу исправления/рефакторинга.

### 2.2. Non-functional

- **Determinism:** одинаковый snapshot → одинаковый результат (порядок, ID).
- **Budgeted output:** ограничение размера выдачи (`budget_tokens`/`budget_chars`) с аккуратным `truncated=true`.
- **Incremental correctness:** per-file snapshots (hash/version) без смешивания состояний.
- **No write:** MCP не пишет файлы и не применяет патчи (read-only).
- **Security:** sandbox roots + защита от path traversal + лимиты на чтение.
- **Observability:** trace/stats по стадиям (load types / parse / IR / resolve / pack).

---

## 3) Компоненты и границы (high-level)

```
Host (LLM)  ──tools/call──>  bsl-agent (MCP, stdio)
                               │
                               ├─ WorkspaceSessionManager (sessions)
                               │     ├─ Policy (roots/include/exclude/limits)
                               │     ├─ WorkspaceProvider (FS read/list/search)
                               │     └─ SemanticProvider (in-proc)
                               │            └─ SemanticFacade (shared API)
                               │                  ├─ TypeRepository (platform + metadata)
                               │                  ├─ ParserCoordinator (AST/IR)
                               │                  └─ Index/Snapshots (symbols/refs/diag)
                               │
                               └─ ContextPackBuilder (budgeted aggregator)
```

### 3.1. WorkspaceSessionManager

Ответственность:

- управление lifecycle сессии (open/status/close);
- кэширование тяжёлых данных (platform types, metadata, индексы);
- выдача “snapshot” контента для анализа.

Данные сессии (минимум):

- `roots[]` (sandbox);
- `platform_docs_archive` + `platform_version` (если задано);
- `configuration_path` (опционально, для metadata);
- runtime кэши и индексы (AST/IR caches, symbol index, diag index).

### 3.2. Policy

Обязательные правила:

- разрешены только пути внутри `roots[]`;
- лимит размера файла (например `max_file_bytes` по умолчанию 1 MiB, настраиваемо);
- лимит результатов поиска/референсов;
- опциональная redaction секретов (простые regex: ключи, токены).

### 3.3. SemanticFacade (shared ядро)

**Цель фасада:** единая, тестируемая, in-proc API, которую используют и LSP handlers, и MCP tools.

Фасад работает по схеме:

- вход: `WorkspaceSnapshot` + `DocumentSnapshot` (path + text + version/hash)
- выход: чистые DTO (diagnostics, symbols, types, members) + минимальные “explain” поля для LLM.

---

## 4) Модель данных и детерминизм

### 4.1. Stable IDs

Нужны для дозапросов и итеративной работы LLM.

- `session_id`: UUID v4.
- `document_id`: нормализованный относительный путь от root (posix).
- `symbol_id`: hash(`document_id|kind|range`).
- `diagnostic_id`: hash(`document_id|range|code|message`).
- `pack_item_id`: hash(`pack_id|kind|primary_key`).

Хеш: `blake3` → hex (короткий, детерминированный).

### 4.2. Sorting

Везде фиксированный порядок:

- сначала по `document_id`, затем по `range.start`, затем по `kind/name`.
- при equal — по `id`.

---

## 5) `context.pack`: как “LLM-friendly” агрегатор

### 5.1. Вход (идея)

- `goal`: короткая цель (“исправить диагностику X”, “понять тип”, “почему completion пустой”).
- `focus`: одно из:
  - `diagnostic_id`
  - `symbol_id`
  - `file + position_utf16 (+ text?)`
  - `query` (строка поиска)
- `budget`: `budget_tokens` (default 1800) и/или `budget_chars` (default ~7000).
- `include`: флаги (snippets/diagnostics/types/references/metadata/coverage/impact).

### 5.2. Выход (идея)

- `text`: готовый для LLM “пакет” (строго в бюджете).
- `items[]`: структурированные элементы (каждый можно расширить через `context.expand`).
- `completeness`: `full|partial`.
- `missing_inputs[]`: что нужно догрузить/разрешить (например “нет platform docs”, “не настроен configuration_path”).

### 5.3. Политика наполнения (defaults)

По умолчанию (без явных include):

1) проектный контекст (platform version, config path, режимы) — 5-10 строк;
2) фокус (snippet 60-120 строк с маркерами диапазонов);
3) связанные diagnostics (тот же файл + “соседи”);
4) типы/члены для выражения под курсором (если focus=position);
5) определение ключевого символа (если находится) + 3-10 наиболее релевантных references.

---

## 6) Что считать “проектом” (scope)

**Решение:** проект — это workspace roots + (опционально) configuration dump.

Scopes:

- `scope=project`: все `**/*.bsl` в roots (с ограничениями/кэшем).
- `scope=file`: один файл.
- `scope=hot`: набор “активных” файлов (передаётся явно) — быстрый default для IDE-host’ов.

---

## 7) Ограничения и деградации (честные ответы)

Примеры `completeness=partial`:

- нет `platform_docs_archive` → доступны только fallback типы/heuristics;
- нет `configuration_path` → нет metadata типов (`Документы.`, `Справочники.` и т.д.);
- файл слишком большой или бинарный → возвращаем “preview + truncated”.

Важно: деградации должны быть явными и объясняемыми в DTO (`reasons[]`).

