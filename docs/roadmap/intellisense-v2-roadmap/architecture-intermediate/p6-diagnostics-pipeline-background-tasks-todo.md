# P6: TODO list — Diagnostics pipeline (syntax + semantic) как фоновые задачи

**Дата:** 2026-01-09  
**Статус:** 🟢 DONE  
**Основание:** Фаза P6 из `docs/roadmap/intellisense-v2-roadmap/architecture-intermediate/salsa-migration-plan.md`

## Цель P6

- Убрать вычисление диагностик (синтаксис + семантика) из hot path `didOpen/didChange` (не блокировать обработку LSP событий).
- Гарантировать freshness: публиковать диагностики только для актуальной версии документа (`file_version`).
- Устранить mixed deps для семантических диагностик: результаты зависят от `deps_id` и используют только deps snapshot.

## Контракт (инварианты)

- **Non-blocking didChange:** `didChange` обновляет inputs и планирует работу, но не ждёт вычисления диагностик.
- **Freshness gate:** публикация происходит только если результат относится к текущему `file_version` и текущим `deps_id/settings_id`.
- **No mixed deps:** семантические диагностики вычисляются от `(file_text, file_path, deps_snapshot, settings_snapshot)` и обязаны наблюдать `deps_id`.
- **Cancellation-friendly:** устаревшие задачи отменяются (или их результаты игнорируются); `salsa::Cancelled` не приводит к публикации.
- **Determinism:** одинаковые `(file_text, file_version, deps_id, settings_id)` → одинаковые диагностики (включая порядок).
- **No I/O in diagnostics:** вычисление диагностик не читает диск/сеть; всё внешнее приходит через inputs/снапшоты.

## Внешние референсы (prior art)

- LSP `textDocument/publishDiagnostics` (включая поле `version`):
  - https://microsoft.github.io/language-server-protocol/specifications/lsp/3.18/specification/#textDocument_publishDiagnostics
  - (комментарии по поддержке `version` клиентом) https://lsp-devtools.readthedocs.io/en/latest/capabilities/text-document/publish-diagnostics.html
- rust-analyzer (подход “диагностики в фоне”):
  - https://rust-analyzer.github.io/book/diagnostics.html
  - https://rust-analyzer.github.io/book/contributing/architecture.html
- Salsa cancellation (результат отмены):
  - https://docs.rs/salsa/latest/salsa/enum.Cancelled.html
- Prior art про отмену ongoing queries при обновлении inputs (идея “set cancels queries”):
  - https://docs.rs/apollo-compiler/latest/apollo_compiler/trait.InputDatabase.html

## Локальные референсы (в репо)

- Текущий sync pipeline диагностик (legacy): `backend/src/bin/lsp_server/handlers/text_document.rs` (`handle_did_open/handle_did_change`).
- Публикация диагностик в LSP: `backend/src/bin/lsp_server/server/language_server.rs` (`did_open/did_change` → `publish_diagnostics(..., Some(version))`).
- Конвертеры domain → LSP diagnostics:
  - `backend/src/bin/lsp_server/converters/diagnostics.rs` (`syntax_errors_to_diagnostics`, `semantic_error_to_diagnostic`).
- Семантическая валидация (domain): `backend/src/application/semantic_validation_visitor/*`,
  `backend/src/application/type_system/services/validation_service.rs`.
- v2 snapshot/queries: `analysis-v2/src/lib.rs` (`file_text/file_version/file_path/parse_result/ir/deps_data/deps_id/settings_id`).

## Решения P6 (фиксируем перед кодом)

### 1) Где живут вычисления диагностик

- `bsl-analysis-v2` предоставляет **query-like API** для диагностик:
  - `syntax_diagnostics(FileId) -> ...` (синтаксис),
  - `semantic_diagnostics(FileId) -> ...` (семантика, зависит от `deps_id`).
- LSP слой:
  - планирует фоновые задачи,
  - запрашивает диагностики из v2 снапшота,
  - конвертит в `tower_lsp::lsp_types::Diagnostic`,
  - публикует только если версия актуальна.

### 2) Разделение “синтаксис” vs “семантика”

Рекомендуемая политика (совместима с текущим `validate_semantics_with_file_path`):
- если есть синтаксические ошибки (и они не “только в директивах”), публикуем только синтаксис и пропускаем семантику;
- если синтаксических ошибок нет — публикуем синтаксис (пусто) + семантику;
- (опционально) если ошибки только в директивах (`&...`) — семантику считаем (best-effort).

### 3) Отмена и коалесинг

- per-file “последняя задача” (ключ = `FileId`/URI):
  - новая версия → `abort()`/cancel предыдущего `JoinHandle`,
  - публикация результатов только если версия совпала (даже если abort не успел).
- (опционально) debounce 30–100ms для серии быстрых `didChange` (экономия CPU).

### 4) Freshness: как избежать “прыжков назад”

- В задачу захватываем `expected_version`.
- Перед `publishDiagnostics` сравниваем `expected_version` с текущим `file_version` в хосте.
- В `publishDiagnostics` передаём `Some(expected_version)` (клиент может дополнительно отфильтровать устаревшее).

## TODO list (реализация)

### A) `analysis-v2`: queries для диагностик

- [x] Определить формат результатов:
  - [x] `syntax_diagnostics(FileId) -> Arc<Vec<ParseError>>` (domain `bsl_shared::domain::types::ParseError`),
  - [x] `semantic_diagnostics(FileId) -> Arc<Vec<TypeDiagnostic>>` (domain `TypeDiagnostic`).
- [x] Учесть требования salsa к return types:
  - [x] ввести snapshot-обёртки + `salsa::Update` (по аналогии с `ParseResultSnapshot`/`SemanticProgramSnapshot`),
  - [x] выбрать стратегию Update и зафиксировать её для диагностик.
- [x] Добавить `#[salsa::tracked]` query `syntax_diagnostics(db, file, settings)`:
  - [x] зависимость от `settings_id` (как минимум для инвалидации),
  - [x] источник данных = `parse_result(...).syntax_errors` (без I/O),
  - [x] порядок детерминирован (как минимум в рамках parser output).
- [x] Добавить `#[salsa::tracked]` query `semantic_diagnostics(db, file, deps, settings)`:
  - [x] **обязательная** зависимость от `deps.id(db)` (исключает mixed deps),
  - [x] gate по синтаксическим ошибкам (см. “Разделение синтаксис/семантика”),
  - [x] источник IR = `ir(file, deps, settings)`,
  - [x] стабильный порядок диагностик (sort по span + severity + message).
- [x] Добавить методы в `AnalysisV2`:
  - [x] `syntax_diagnostics(file_id) -> Cancellable<Option<Arc<Vec<ParseError>>>>`,
  - [x] `semantic_diagnostics(file_id) -> Cancellable<Option<Arc<Vec<TypeDiagnostic>>>>`.
- [x] Unit tests (минимум) в `analysis-v2`:
  - [x] синтаксические ошибки возвращаются и детерминированы,
  - [x] семантические ошибки зависят от `deps_id`,
  - [x] `salsa::Cancelled` пробрасывается как `Err(Cancelled)` (см. `cancellable` helper).

### B) Семантическая диагностика: разрулить границы крейтов

`analysis-v2` не должен зависеть от `bsl-backend`, поэтому для `semantic_diagnostics` нужно вынести логику из backend.

- [x] Выделить новую workspace-crate: `bsl-semantic-diagnostics`:
  - [x] перенести `backend/src/application/semantic_validation_visitor/*` (и минимальные зависимости),
  - [x] API: `pub use SemanticValidationVisitor` (domain-level),
  - [x] никаких ссылок на LSP типы/клиент, только domain (`bsl_shared`).
- [x] Подключить новый crate в `analysis-v2` и `backend`:
  - [x] `analysis-v2` использует его внутри query `semantic_diagnostics`,
  - [x] backend держит compatibility re-export (без циклических зависимостей).

### C) Backend LSP: фоновые задачи и публикация

- [x] Добавить scheduler состояния (per-file handles) в `BslLanguageServer`:
  - [x] `HashMap<V2FileId, (expected_version, JoinHandle<()>)>` + `abort()` при новой версии,
  - [x] очистка на `didClose`.
- [x] Подключить планирование на `didOpen/didChange` (v2 флаг):
  - [x] `didOpen/didChange`: обновить inputs → `schedule_diagnostics_v2(uri, file_id, expected_version)`.
- [x] Реализовать `schedule_diagnostics_v2`:
  - [x] взять snapshot “быстро” (не держать lock на время вычислений),
  - [x] вычислить diagnostics (syntax → возможно semantic),
  - [x] сверить актуальность `file_version` и `deps_id/settings_id` перед publish,
  - [x] `publish_diagnostics(uri, diagnostics, Some(expected_version))`.
- [x] Не словить non-Send future:
  - [x] не держать `AnalysisV2`/snapshot через `.await` (сначала вычислить в sync части, потом публиковать).
- [x] Фильтровать/настраивать вывод:
  - [x] уважать `settings.diagnostics.show_hints` (не публиковать `Hint`, если выключено).
- [x] Логи/наблюдаемость:
  - [x] логировать `observed (file_version, deps_id, settings_id)` для опубликованных диагностик,
  - [x] логировать причины пропуска (cancelled, stale version, stale deps/settings, no file).

### D) Верификация (обязательная)

- [x] Тест: быстрые серии `didChange` → publishDiagnostics не регрессирует по версии:
  - [x] симулировать `didOpen` + серию `didChange` с версиями,
  - [x] проверить монотонность `PublishDiagnosticsParams::version` (после N не приходит N-1).
- [x] `cargo test --workspace` проходит.

## DoD (P6 считается закрытым, если)

- [x] `didOpen/didChange` не выполняют синтаксическую/семантическую валидацию синхронно; только обновляют inputs и планируют фоновые задачи.
- [x] Реализованы `syntax_diagnostics(FileId)` и `semantic_diagnostics(FileId)` (семантика зависит от `deps_id`).
- [x] Публикация диагностик гейтится по `file_version` (и результаты не “прыгают назад”).
- [x] Есть тест, покрывающий быстрые серии `didChange`.

## Верификация (факты)

- ✅ `analysis-v2/src/lib.rs`: реализованы `syntax_diagnostics`/`semantic_diagnostics` + `AnalysisV2::{syntax_diagnostics, semantic_diagnostics}`.
- ✅ `backend/src/bin/lsp_server/server/core.rs`: реализован `schedule_diagnostics_v2` (cancel + freshness gate) и тест `p6_fast_did_change_series_publish_diagnostics_is_monotonic`.
- ✅ `cargo test --workspace` — OK.

## Долги (после P6)

- [x] Протянуть `settings.diagnostics.detail_level` в v2 semantic diagnostics (нужно расширить `SettingsSnapshot`, сейчас хранит только `settings_id`).
