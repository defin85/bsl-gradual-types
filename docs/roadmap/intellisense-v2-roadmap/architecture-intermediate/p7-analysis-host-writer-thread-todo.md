# P7: TODO list — AnalysisHost v2 как отдельный writer thread (ra-style)

**Дата:** 2026-01-10  
**Статус:** 🟢 DONE  
**Основание:** Фаза P7 из `docs/roadmap/intellisense-v2-roadmap/architecture-intermediate/salsa-migration-plan.md`

## Цель P7

- Убрать `Mutex`/`RwLock` вокруг mutable salsa DB в LSP слое: **только один поток** владеет `AnalysisHostV2` и применяет изменения.
- Снизить contention и риск “mixed state” на уровне синхронизации: изменения текста/версии/зависимостей применяются последовательно.
- Сделать протокол “как у rust-analyzer”: `AnalysisHost` (mutable, apply_change) + `Analysis` (immutable snapshot), который читают запросы.

## Контракт (инварианты)

- **Single writer:** только writer thread имеет доступ к `&mut AnalysisHostV2` и вызывает `apply_change`.
- **No DB locks in LSP:** LSP обработчики не держат lock на время вычислений; они работают на `AnalysisV2` снапшоте.
- **Snapshot lifetime:** снапшот не живёт через `.await` и не хранится глобально; снапшот используется как “request-local”.
- **Freshness/observability:** каждый ответ (completion/hover/signatureHelp/diagnostics) логирует observed контекст:
  `observed (file_version, deps_id, settings_id)` (и при необходимости `FileId`).
- **Latency monitoring:** time-to-ready (`wait_for_file_version`), `snapshot()` и ключевые queries измеряются и могут логировать slow-path по порогам; это помогает ловить ситуации, когда активные snapshots блокируют writes и запросы ждут дольше ожидаемого.
- **Cancellation-friendly:** запросы/таски корректно обрабатывают `salsa::Cancelled` (не публикуют устаревшее).

Важно: в salsa **любой write** (изменение inputs) триггерит cancellation и может **блокировать** writer thread,
пока существуют активные snapshots (см. `salsa` book “Database and runtime” / “Incrementing the revision counter”).
Это нормально, но требует дисциплины по времени жизни снапшотов и (по возможности) `db.unwind_if_revision_cancelled()`
в тяжёлых участках вычислений.

## Внешние референсы (prior art)

- rust-analyzer архитектура (`AnalysisHost` / `Analysis` snapshot):
  - https://rust-analyzer.github.io/book/contributing/architecture.html
- salsa 0.25: модель master DB + snapshots и блокировка write при активных snapshots:
  - https://docs.rs/salsa/0.25.2/salsa/
  - (исходник/книга в crate) `salsa-0.25.2/book/src/plumbing/database_and_runtime.md`
- salsa cancellation payload:
  - https://docs.rs/salsa/latest/salsa/enum.Cancelled.html

## Локальные референсы (в репо)

- Текущий v2 host (mutable) + snapshot API:
  - `analysis-v2/src/lib.rs` (`AnalysisHostV2::{apply_change,snapshot}`, `AnalysisV2`)
- Runtime writer thread (P7):
  - `backend/src/bin/lsp_server/server/analysis_v2_runtime.rs` (`AnalysisV2Runtime`)
- Проводка в LSP (P7):
  - `backend/src/bin/lsp_server/server/mod.rs` (`analysis_v2: AnalysisV2Runtime`, `latest_received_file_versions_v2`)
  - `backend/src/bin/lsp_server/server/core.rs` (`sync_v2_globals`, `schedule_diagnostics_v2`)
  - `backend/src/bin/lsp_server/server/language_server.rs` (didOpen/didChange/didClose + барьер для completion/hover/signatureHelp)

## Решения P7 (фиксируем перед кодом)

### 1) Где живёт writer thread и какой протокол

Рекомендация: держать writer thread в LSP слое (crate `backend`, рядом с сервером), как “actor”:

- **Вход:** очередь команд (events), которые изменяют состояние v2:
  - `didOpen/didChange/didClose` → `Change::{SetFile,RemoveFile}`
  - `deps_update` → `Change::SetDepsSnapshot`
  - изменения настроек → `Change::SetSettingsSnapshot`
- **Выход:** on-demand снапшоты `AnalysisV2` для запросов.

Writer thread выполняет только:
- применение изменений (быстро),
- выдачу снапшотов (быстро),
а все тяжёлые вычисления (queries + дальнейшая доменная логика) выполняются в запросных тасках на снапшоте.

### 2) Каналы/очереди

Реализация:
- очередь команд: `std::sync::mpsc::Sender/Receiver` (blocking loop в std thread),
- ответы: `tokio::sync::oneshot` (удобно ждать из async кода).

### 3) Правила “no deadlocks”

- Не делать `await`, пока в стеке живёт `AnalysisV2` снапшот.
- Не хранить `AnalysisV2` в `ArcSwap`/глобальном кеше: активный снапшот может блокировать writes.
- Не выполнять write (apply_change) в том же потоке, где есть живой снапшот (salsa прямо предупреждает о дедлоке).

### 4) Протокол запросов (минимум)

- LSP запрос формирует request-local snapshot, извлекает нужные данные, дропает snapshot, затем делает async работу.
- (Опционально, но желательно) унифицировать observed контекст:
  - `AnalysisV2::observed_context(file_id) -> Cancellable<Option<ObservedContext>>`,
  - где `ObservedContext` = `{ file_version, deps_id, settings_id }`.

### 5) Барьер на запросе (выбран вариант B)

**Решение:** `didChange` **не ждёт** применения изменений; барьер делаем на запросе через “flush/wait”.

Мотивация: минимизировать задержку обработки `didChange` и позволить коалесинг серии изменений; корректность обеспечиваем тем,
что запросы ждут применения нужной версии перед чтением снапшота.

Минимальный протокол:

- LSP слой ведёт `latest_received_version[file_id]` (обновляется при получении `didOpen/didChange`).
- Перед обработкой completion/hover/signatureHelp/diagnostics (v2 ветка) делаем:
  1) читаем `expected_version = latest_received_version[file_id]`,
  2) `analysis_v2.wait_for_file_version(file_id, expected_version).await`,
  3) `analysis_v2.snapshot().await` и только потом queries/логика.
- Если во время вычислений пришла новая версия (expected изменился), допускается:
  - либо 1 быстрый retry (пере-взять expected + wait + snapshot),
  - либо early-return/skip publish (как уже делаем для диагностик по freshness gate).

Важно: `wait_for_file_version` в writer thread **не должен блокировать** обработку очереди; он должен парковать запрос
и продолжать применять входящие `ApplyChanges`.

Сравнение с альтернативой (вариант A):

- **A: ACK/flush на `didChange`:** `didChange` ждёт подтверждение применения изменений в writer thread.
  - Плюсы: запросы почти всегда сразу берут снапшот (меньше ожиданий в `completion/hover`).
  - Минусы: `didChange` становится “дорогим” и может блокироваться, особенно если в момент применения есть активные snapshots.
- **B: fire-and-forget на `didChange` + барьер на запросе (выбран):**
  - Плюсы: `didChange` максимально быстрый, легче коалесить серию изменений.
  - Минусы: запрос может подождать, пока writer применит нужную версию; нужен явный `wait_for_file_version`.

## TODO (P7)

### 0) Добавить абстракцию “analysis runtime” (writer thread + handle)

- [x] Создать модуль `backend/src/bin/lsp_server/server/analysis_v2_runtime.rs` с типами:
  - [x] `AnalysisV2Runtime` (handle, cloneable),
  - [x] `AnalysisV2Runtime::new(initial_host: AnalysisHostV2, initial_index_snapshot: Arc<IndexSnapshot>) -> Self` (spawn writer thread),
  - [x] `AnalysisV2Runtime::apply_changes(Vec<bsl_analysis_v2::Change>)` (fire-and-forget enqueue),
  - [x] `AnalysisV2Runtime::snapshot() -> impl Future<Output = AnalysisV2>`,
  - [x] `AnalysisV2Runtime::wait_for_file_version(file_id: FileId, min_version: i32) -> impl Future<Output = bool>`,
  - [x] `shutdown_for_test()` для тестов (чистое завершение потока).
- [x] Определить enum команд для writer thread:
  - [x] `ApplyChanges { changes }`,
  - [x] `GetSnapshot { reply }`,
  - [x] `WaitForFileVersion { file_id, min_version, reply }`,
  - [x] `Shutdown { ack }` (test-only).

### 1) Перевести `BslLanguageServer` на runtime вместо `Arc<Mutex<AnalysisHostV2>>`

- [x] `backend/src/bin/lsp_server/server/mod.rs`:
  - [x] заменить поле `Arc<Mutex<AnalysisHostV2>>` на `analysis_v2: AnalysisV2Runtime`.
- [x] `backend/src/bin/lsp_server/server/core.rs`:
  - [x] при старте сервера создать initial host (deps/settings) и запустить runtime,
  - [x] `sync_v2_globals` переписать на `analysis_v2.apply_changes(...)` вместо `lock + apply_change`.

### 2) LSP события документов: все writes через runtime

- [x] `didOpen/didChange/didClose` (v2 ветка):
  - [x] отправлять `Change::{SetFile,RemoveFile}` в writer thread,
  - [x] не делать тяжёлой работы в обработчике события.
- [x] Ввести в LSP слое `latest_received_version[file_id]`:
  - [x] обновлять в `didOpen/didChange` синхронно с применением `contentChanges`,
  - [x] удалять/сбрасывать в `didClose`.

### 3) Запросы LSP: снапшоты только request-local

- [x] Completion/hover/signatureHelp v2 ветки:
  - [x] `expected_version = latest_received_version[file_id]`,
  - [x] `analysis_v2.wait_for_file_version(file_id, expected_version).await`,
  - [x] вместо `analysis_host_v2.lock().await.analysis()` → `analysis_v2.snapshot().await`,
  - [x] извлечь данные синхронно,
  - [x] дропнуть снапшот до `.await`.
- [x] Diagnostics pipeline v2 (`schedule_diagnostics_v2`):
  - [x] перед вычислением: `analysis_v2.wait_for_file_version(file_id, expected_version).await`,
  - [x] заменить получение снапшота и “current state” проверки на `analysis_v2.snapshot().await`.

### 4) Наблюдаемость и “observed контекст”

- [x] Для всех v2 публикаций/ответов:
  - [x] логировать `observed (file_version, deps_id, settings_id)` и `FileId`,
  - [x] логировать причины skip (cancelled / stale version / stale deps/settings / no file).

### 5) Тесты (минимум)

- [x] Unit tests для runtime:
  - [x] apply_changes меняет состояние, снапшот видит изменения,
  - [x] `wait_for_file_version` не блокирует writer loop и корректно “просыпается” после SetFile,
  - [x] shutdown завершает поток без зависаний (test-only).
- [x] Интеграционный smoke test (в `backend/src/bin/lsp_server/server/core.rs` рядом с P6 тестом):
  - [x] серия `didOpen` + `didChange` + `completion` (v2) не приводит к deadlock/timeout.

## DoD (P7 считается закрытым, если)

- [x] В LSP сервере нет `Arc<Mutex<AnalysisHostV2>>`; все изменения идут через writer thread runtime.
- [x] Все v2 LSP фичи используют только request-local `AnalysisV2` snapshot и не держат его через `.await`.
- [x] В v2 запросах есть “барьер” на нужную версию (`wait_for_file_version`), `didChange` не ждёт применения изменений.
- [x] `cargo test --workspace` проходит.

## Ручная проверка (рекомендовано)

- [ ] Нагрузочный сценарий: быстрый ввод текста + частые completion/hover → нет “подвисаний”/deadlock и нет mixed state.

## Верификация (факты)

- ✅ `backend/src/bin/lsp_server/server/analysis_v2_runtime.rs`: `AnalysisV2Runtime` + тесты `p7_apply_changes_and_wait_for_version_works` и `p7_waiters_are_released_on_shutdown`.
- ✅ `backend/src/bin/lsp_server/server/language_server.rs`: `didOpen/didChange/didClose` (fire-and-forget) + барьер `wait_for_file_version` в completion/hover/signatureHelp.
- ✅ `backend/src/bin/lsp_server/server/core.rs`: `sync_v2_globals` через `analysis_v2.apply_changes`, `schedule_diagnostics_v2` с барьером + тест `p7_completion_after_did_change_does_not_hang`.
- ✅ `backend/src/system/basic_observability.rs` + `backend/src/system/system_coordinator/coordinator.rs`: метрики `intellisense_v2_*` для wait/snapshot/query latency.
- ✅ `backend/src/bin/lsp_server/server/mod.rs`: пороги warn по env vars `BSL_INTELLISENSE_V2_SLOW_WAIT_WARN_MS`, `BSL_INTELLISENSE_V2_SLOW_SNAPSHOT_WARN_MS`, `BSL_INTELLISENSE_V2_SLOW_QUERY_WARN_MS`.
- ✅ `cargo test -p bsl-backend --bin bsl-lsp-server p7_` — OK (3/3).
- ✅ `cargo check -p bsl-backend --bin bsl-lsp-server` — OK.
- ✅ `cargo test --workspace` — OK.
