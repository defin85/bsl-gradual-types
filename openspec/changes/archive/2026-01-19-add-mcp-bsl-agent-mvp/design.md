# Design: `bsl-agent` (MCP) — MVP локального семантического агента

## Референсы

- Архитектура (roadmap): `docs/roadmap/mcp-bsl-agent/architecture.md`
- MCP API (roadmap): `docs/roadmap/mcp-bsl-agent/api.md`
- План работ (roadmap): `docs/roadmap/mcp-bsl-agent/implementation-plan.md`
- Паттерн MCP реализации: `mcp-debug-server/`

Этот документ фиксирует “минимально достаточные” решения для MVP и служит ссылкой при реализации задач из `tasks.md`.

## Ключевые решения (MVP)

### 1) Семантика: in-proc, без проксирования через LSP

MCP‑сервер предоставляет семантику через общий `SemanticFacade`/`SemanticProvider` внутри процесса.

Причины:
- текущий LSP сервер ориентирован на режим `stdio` и состояние документов внутри сессии клиента;
- проксирование добавляет отдельный процесс/синхронизацию `didOpen/didChange` и снижает детерминизм;
- in-proc даёт прямой контроль над snapshot’ами, кэшами и лимитами.

### 2) Read-only и безопасность

MVP не модифицирует workspace:
- нет операций записи/патчей/рефакторинга;
- доступ к файлам ограничен roots sandbox (canonicalize + защита от path traversal);
- вводятся лимиты на размер/объём чтения и число результатов на запрос.

### 3) Unsaved buffers: ad-hoc + session overlay

Поддерживаются два режима:
- **ad-hoc snapshot**: `FileRef.text` в конкретном tool-call (анализ текста только в рамках этого вызова);
- **session overlay**: `workspace_documents_set` / `workspace_documents_clear` хранит unsaved тексты в памяти сессии (для `scope=hot` и `context_pack`).

Любое изменение effective‑состояния документов увеличивает `analysis_revision` (монотонный `u64`).

### 4) Детерминизм: stable ids + sorting + бюджетирование

Для повторяемости и дозапросов:
- все ответы возвращают `analysis_revision`;
- идентификаторы (`symbol_id`, `diagnostic_id`, `pack_id`, `item_id`) стабильны **внутри одного `analysis_revision`**;
- порядок результатов фиксирован (document → range → kind/name → id);
- `context_pack` и сниппеты работают в рамках жёсткого лимита `budget_chars` и явно сигналят `truncated=true`.

### 5) API и слой DTO

DTO намеренно не зависят от `tower-lsp` типов:
- позиции/диапазоны совместимы с LSP: `Position.character` — UTF‑16;
- ответы MCP — компактные JSON DTO (см. `docs/roadmap/mcp-bsl-agent/api.md`).

### 6) Локальный кэш: процесс взаимодействия (DiskCache/AstCache)

Цель кэша в MVP: ускорить повторные открытия workspace и избежать дорогой переработки платформенных/конфигурационных данных между запусками `bsl-agent`.

**Какие кэши используем**
- **DiskCache (межпроцессный, на диске):** для тяжёлых артефактов, которые безопасно переиспользовать между сессиями/процессами:
  - platform dataset (`platform`, `platform_raw`)
  - config metadata и производные индексы (`config*`, `combined`, и т.п. — как в существующем `backend/src/system/system_coordinator/*`)
  - опционально: AST disk cache (`source_kind=ast`) для ускорения парсинга модулей
- **AstCache (в памяти, LRU):** быстрый L1 кэш парсинга BSL по хешу контента (управляется через `BSL_AST_CACHE_CAPACITY`).

**Где лежит кэш**
- По умолчанию `DiskCache` использует `${XDG_CACHE_HOME}/bsl-gradual-types` или `~/.cache/bsl-gradual-types`.
- Переопределение: `BSL_CACHE_DIR`.
- Отключение: `BSL_CACHE_DISABLE=1|true|yes`.
- Доп. политики (если понадобятся): TTL/cleanup/SWR через `BSL_CACHE_TTL_*`/`BSL_CACHE_SWR` и др. (см. `backend/src/system/disk_cache.rs`).

**Что значит “read-only” в контексте кэша**
- `bsl-agent` не изменяет файлы проекта в roots (никаких write/patch).
- Запись допускается только в директорию кэша (вне roots) и только для производных артефактов (manifest + бинарные payload’ы).

**Процесс на `workspace_open`**
1) Создать/сконфигурировать `DiskCache` (schema_version = 1, как в backend) с учётом env.
2) Если указан `platform_docs_archive`/путь к платформенной документации:
   - построить cache key на основе `source_identity` (канонический путь), `source_fingerprint` (зависит от strict режима) и `settings_fingerprint` (включая версию платформы);
   - попытаться `get_or_build_with_swr`; при miss — построить dataset и сохранить.
3) Если указан `configuration_path`:
   - построить cache key’и для discovery/metadata и layer‑B индексов (как в `SystemCoordinator`);
   - попытаться reuse через `DiskCache`; при miss — построить и сохранить.
4) Привязать `DiskCache` к парсингу модулей (AST disk cache) и выставить `cache_scope` (`project_id`/`config_id`) когда они известны.
5) `workspace_status` отражает прогресс (`loading_platform`/`loading_config`/`indexing`), но детали hit/miss в MVP можно оставлять в логах; отдельная телеметрия/DTO — опционально (не обязателен для MVP).

**Процесс на `workspace_documents_set/clear`**
- Overlay хранится только в памяти сессии и увеличивает `analysis_revision`.
- Кэш на диске не должен сохранять “сырой” unsaved текст; допускается только кэширование производных артефактов (например, AST) без восстановления исходного текста.

### 7) Конкурентный доступ к кэшу (LSP + MCP)

Цель: LSP‑сервер и `bsl-agent` должны безопасно разделять один `DiskCache` без повреждения артефактов и без “гонок” при одновременной сборке.

**Правило:** `bsl-agent` использует тот же `DiskCache`/`DiskCacheKey` и схему ключей, что и backend (для совместного reuse), а конкурентный доступ решается на уровне реализации `DiskCache`.

Ожидаемое поведение:
- На один cache entry действует файловая блокировка `.lock` (per‑key). При одновременном `get_or_build*`:
  - один процесс строит/пишет,
  - остальные ждут lock и затем читают валидный артефакт.
- Запись артефактов атомарная (`tmp + rename`), поэтому читатели не видят “полузаписанные” файлы.

**Критичный момент:** операции удаления (TTL/eviction/cleanup) не должны удалять entry, пока на нём удерживается `.lock` другим процессом. Иначе возможна ситуация “удалили `.lock` во время удержания lock” → второй процесс создаст новый `.lock` и обойдёт взаимное исключение.

Для MVP фиксируем требование:
- cleanup/eviction должны пытаться захватить `.lock` перед удалением и пропускать entry, если lock удерживается.

## Границы и не‑цели

В MVP не делаем:
- remote режим и синхронизацию с сервером семантики;
- MCP transport кроме stdio;
- расширенные resources/prompts, не необходимые для `context_pack`.

## План реализации

Подробный чеклист — в `openspec/changes/add-mcp-bsl-agent-mvp/tasks.md`.
