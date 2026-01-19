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

**Решение:** основной интерфейс для LLM — агрегирующий tool `context_pack`, который:

- принимает “цель” (goal) и “фокус” (diagnostic/symbol/file+pos/query);
- возвращает компактный текстовый пакет + структурированный список items;
- работает **в рамках бюджета** (chars/tokens) и умеет честно отвечать `completeness=partial` + `missing_inputs[]` при нехватке данных.

Точечные tools (`bsl_diagnostics`, `bsl_type_at_position`, `bsl_definition`, `bsl_references`, `bsl_members`, `bsl_symbol_search`) нужны только для дозапросов.

### 1.3. Local-first (на данном этапе)

**Решение:** первая версия — **локальная**:

- MCP читает FS workspace напрямую (roots sandbox).
- Семантика строится in-proc, без “тонкого агента” к remote Semantic Server.

**Заложено на будущее:** интерфейс `SemanticProvider` позволяет добавить remote режим позже (без смены MCP API).

### 1.4. Unsaved buffer из IDE

**Решение:** поддержать двумя уровнями (оба read-only, без записи на диск):

- **ad-hoc snapshot:** `FileRef.text` в конкретном tool-call (анализируем текст только в рамках одного вызова);
- **session overlay:** инструменты `workspace_documents_set` / `workspace_documents_clear` сохраняют unsaved тексты в памяти сессии, чтобы `scope=hot` и `context_pack` работали по реальному состоянию редактора.

Каждое изменение effective-состояния документов (overlay и/или изменения на диске, обнаруженные сервером) увеличивает `analysis_revision` (монотонный `u64`). Все семантические ответы возвращают `analysis_revision`; ID (`symbol_id`, `diagnostic_id`, `pack_id`, `item_id`) валидны только в рамках конкретного `analysis_revision`.

---

## 2) Цели и нефункциональные требования

### 2.1. Functional

- `bsl_diagnostics` по файлу и по проекту (с группировками).
- “семантическая навигация”:
  - `bsl_type_at_position`
  - `bsl_members` (member list / completion-like для receiver выражений)
  - `bsl_definition`
  - `bsl_references`
  - `bsl_symbol_search`
- `context_pack`: собрать контекст “готовый для LLM” под задачу исправления/рефакторинга.

### 2.2. Non-functional

- **Determinism:** одинаковый snapshot → одинаковый результат (порядок, ID).
- **Budgeted output:** ограничение размера выдачи (`budget_chars` как hard limit; `budget_tokens` как подсказка/alias) с аккуратным `truncated=true`.
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
                               │     ├─ DocumentStore (disk + overlays + hot set)
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
- выдача “snapshot” контента для анализа (disk + overlays).
- управление `analysis_revision` и “hot” набором документов (IDE‑friendly).

Данные сессии (минимум):

- `roots[]` (sandbox) + стабильные `root_id`;
- `platform_docs_archive` + `platform_version` (если задано);
- `configuration_path` (опционально, для metadata);
- `analysis_revision` (монотонный счётчик изменений overlay);
- runtime кэши и индексы (AST/IR caches, symbol index, diag index).
  - включая кэши, привязанные к `analysis_revision`/hash’ам документов.

### 3.2. Policy

Обязательные правила:

- разрешены только пути внутри `roots[]`;
- лимит размера файла (например `max_file_bytes` по умолчанию 1 MiB, настраиваемо);
- лимит результатов поиска/референсов;
- опциональная redaction секретов (простые regex: ключи, токены).

### 3.3. SemanticFacade (shared ядро)

**Цель фасада:** единая, тестируемая, in-proc API, которую используют и LSP handlers, и MCP tools.

Фасад работает по схеме:

- вход: `WorkspaceSnapshot` + `DocumentSnapshot` (DocumentRef + text + version/hash + source)
- выход: чистые DTO (diagnostics, symbols, types, members, impact/coverage) + минимальные “explain” поля для LLM.

### 3.4. DocumentStore и WorkspaceSnapshot

DocumentStore — ключевой компонент для LLM‑воркфлоу: он отделяет “что лежит на диске” от “что сейчас в редакторе”.

- хранит overlay-тексты (unsaved buffers) и метаданные (`version`, `hash`, `source`);
- умеет отдавать `DocumentSnapshot` по `DocumentRef`:
  - если есть overlay → берём overlay;
  - иначе читаем с диска (в пределах `roots[]`, с лимитами `Policy`).
- поддерживает `hot_set` (активные документы) для `scope=hot` и быстрых pack’ов.

`WorkspaceSnapshot` фиксирует “какие документы и в каких версиях” использовались для конкретного анализа. Все результаты семантики должны ссылаться на `analysis_revision`/hash’и, чтобы LLM мог понимать, что результаты соответствуют конкретному snapshot’у.

---

## 4) Модель данных и детерминизм

### 4.1. Stable IDs

Нужны для дозапросов и итеративной работы LLM.

- `session_id`: UUID v4.
- `root_id`: hash(canonical_abs_root_path) → hex (для multi-root без коллизий).
- `document_id`: `root_id:path` (path нормализован как posix relative path внутри root).
- `analysis_revision`: монотонный `u64` внутри сессии (увеличивается на любые изменения overlay).
- `symbol_id`: hash(`analysis_revision|document_id|kind|range|name?`).
- `diagnostic_id`: hash(`analysis_revision|document_id|range|code|message`).
- `pack_id`: hash(`analysis_revision|goal|focus|include|budget_chars`).
- `pack_item_id`: hash(`pack_id|kind|primary_key`).

Хеш: `blake3` → hex (короткий, детерминированный).

Важно: ID считаются стабильными **внутри одного `analysis_revision`**. Если `analysis_revision` изменился (например, после `workspace_documents_set`), старые `symbol_id/diagnostic_id/pack_id` могут стать stale — сервер должен отвечать явно (ошибка `stale_id`/`stale_revision` или `completeness=partial` + reason).

### 4.2. Sorting

Везде фиксированный порядок:

- сначала по `document_id`, затем по `range.start`, затем по `kind/name`.
- при equal — по `id`.

### 4.3. Budget policy (детерминизм выдачи)

- `budget_chars` — **жёсткий лимит** на текстовые поля (`text`, `snippet.text`) и суммарный `context_pack.text`.
- `budget_tokens` — **не гарантия**, а только подсказка/alias для выставления `budget_chars` по детерминированной формуле (например, фиксированный коэффициент chars-per-token).
- Любая обрезка обязана быть явной: `truncated=true`, плюс причины/счётчики (сколько элементов/строк выкинули).

---

## 5) `context_pack`: как “LLM-friendly” агрегатор

### 5.1. Вход (идея)

- `goal`: короткая цель (“исправить диагностику X”, “понять тип”, “почему completion пустой”).
- `focus`: одно из:
  - `diagnostic_id`
  - `symbol_id`
  - `file + position_utf16 (+ text?)`
  - `query` (строка поиска)
- `scope`: `hot|file|project` (по умолчанию `hot` для IDE-host’ов, `project` для CLI-host’ов).
- `budget`: `budget_chars` (hard limit, default ~7000) и/или `budget_tokens` (alias).
- `include`: флаги (snippets/diagnostics/types/references/metadata/coverage/impact).

### 5.2. Выход (идея)

- `text`: готовый для LLM “пакет” (строго в бюджете).
- `items[]`: структурированные элементы (каждый можно расширить через `context_expand`).
- `analysis_revision`: ревизия snapshot’а, для которой сформирован pack.
- `completeness`: `full|partial`.
- `missing_inputs[]`: что нужно догрузить/разрешить (например “нет platform docs”, “не настроен configuration_path”).

### 5.3. Политика наполнения (defaults)

По умолчанию (без явных include):

1) проектный контекст (platform version, config path, режимы) — 5-10 строк;
2) фокус (snippet 60-120 строк с маркерами диапазонов);
3) связанные diagnostics + impact/coverage summary (“радиус поражения”, непроверенные операции);
4) типы/члены для выражения под курсором (если focus=position);
5) определение ключевого символа (если находится) + 3-10 наиболее релевантных references.

---

## 6) Что считать “проектом” (scope)

**Решение:** проект — это workspace roots + (опционально) configuration dump.

Scopes:

- `scope=project`: все `**/*.bsl` в roots (с ограничениями/кэшем).
- `scope=file`: один файл.
- `scope=hot`: набор “активных” файлов из `DocumentStore.hot_set` (заполняется через `workspace_documents_set(mark_hot=true)`).

---

## 7) Ограничения и деградации (честные ответы)

Примеры `completeness=partial`:

- нет `platform_docs_archive` → доступны только fallback типы/heuristics;
- нет `configuration_path` → нет metadata типов (`Документы.`, `Справочники.` и т.д.);
- файл слишком большой или бинарный → возвращаем “preview + truncated”.
- `symbol_id/diagnostic_id` от старого `analysis_revision` → явный ответ “stale id” + просьба обновить pack/повторить запрос.

Важно: деградации должны быть явными и объясняемыми в DTO (`reasons[]`).
