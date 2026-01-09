# P5: TODO list — перевод completion/hover/signatureHelp на v2

**Дата:** 2026-01-09  
**Статус:** 🟢 DONE  
**Основание:** Фаза P5 из `docs/roadmap/intellisense-v2-roadmap/architecture-intermediate/salsa-migration-plan.md`

## Цель P5

- Перевести `textDocument/completion`, `textDocument/hover`, `textDocument/signatureHelp` на путь **v2 snapshot → queries → результат**, без обращения к legacy-парсингу/IR в hot path.
- Сохранить поведение и качество (по golden/fixture тестам): v2 выдаёт тот же (или ожидаемо совместимый) результат, что и legacy.
- Зафиксировать детерминизм результатов (порядок, `sortText`, подготовка к стабильному `candidate_id`).

## Контракт (инварианты)

- **Single source of truth:** при включённом `BSL_INTELLISENSE_V2_SALSA=1` LSP фичи читают данные только из `AnalysisHostV2::analysis()` (текст/версия/позиционирование/IR/parse_result + observed ids).
- **No legacy IR build:** в v2 пути запрещены `parse_to_ir`, `parse_with_cache_for_file` и `backend/src/system/ir_cache.rs` (можно оставить legacy-only).
- **Deps correctness:** семантика и LSP ответы должны соответствовать одному снапшоту deps (observed `deps_id`) — без mixed deps.
- **Determinism:** одинаковые `(file_text, file_version, deps_id, settings_id)` → одинаковые результаты (включая порядок и `sortText`).
- **Cancellation-friendly:** отмена (salsa `Cancelled`) приводит к пустому/None результату и не публикует устаревшие данные.
- **No I/O in hot path:** completion/hover/signatureHelp не читают диск/сеть; всё внешнее уже превращено в inputs/снапшоты.

## Внешние референсы (prior art)

- LSP Completion/Resolve: `CompletionItem.data` сохраняется между `textDocument/completion` и `completionItem/resolve`,
  а `sortText/filterText/insertText/textEdit` не должны “переезжать” на resolve:
  - https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#textDocument_completion
- Salsa cancellation payload:
  - https://docs.rs/salsa/latest/salsa/enum.Cancelled.html
- Query-based модель вычислений (rustc):
  - https://rustc-dev-guide.rust-lang.org/query.html

## Локальные референсы (в репо)

- v2 host wiring + feature flag:
  - `backend/src/bin/lsp_server/server/core.rs` (`BSL_INTELLISENSE_V2_SALSA`, `sync_v2_globals`)
  - `backend/src/bin/lsp_server/server/language_server.rs` (ветки `if self.use_salsa_v2`)
- Legacy LSP handlers:
  - `backend/src/bin/lsp_server/handlers/completion.rs`
  - `backend/src/bin/lsp_server/handlers/hover.rs`
  - `backend/src/bin/lsp_server/handlers/signature_help.rs`
- Legacy domain logic:
  - `backend/src/application/type_system/services/completion_service.rs`
  - `backend/src/application/type_system/services/hover_service.rs`
- v2 queries / API:
  - `analysis-v2/src/lib.rs` (`line_index/parse_result/ir`, позиционирование)
- Candidate identity (связано с “candidate_id стабильный”):
  - `docs/roadmap/intellisense-v2-roadmap/m6-implementation-plan.md`

## Решения P5 (фиксируем перед кодом)

### 1) Стратегия миграции: strangler под флагом

- `BSL_INTELLISENSE_V2_SALSA=0` → legacy путь без изменений.
- `BSL_INTELLISENSE_V2_SALSA=1` → LSP фичи используют:
  1) `analysis_host_v2.snapshot()`/`analysis()` как источник текста/версии,
  2) v2 queries (`line_index/parse_result/ir`) как источник синтаксиса/IR,
  3) существующую “бизнес-логику” (completion/hover/signatureHelp), но **без** вызовов legacy IR build.

### 2) Где живёт вычисление результата

Рекомендация для P5: вычисление результата остаётся в backend (LSP слой), а `bsl-analysis-v2` остаётся слоем
“inputs → queries” (синтаксис/позиционирование/IR). Это позволяет:
- не тащить backend-зависимости в `analysis-v2`,
- переиспользовать текущие алгоритмы completion/hover/signatureHelp,
- держать переход под runtime feature flag.

### 3) Как избежать mixed deps в LSP фичах

Для v2 пути нельзя брать репозиторий типов/резолвер “сбоку” (из глобального состояния), иначе можно смешать deps.

Решение:
- добавляем публичный доступ к deps data из `AnalysisV2` (минимум: `Arc<SemanticDeps>`),
  и используем его для type resolution/lookup в completion/hover/signatureHelp.
- индекс IntelliSense (backend) допускается читать из coordinator, но **обязательно** сверять `deps_id`/snapshot_id на входе
  и логировать observed ids.

### 4) Determinism и стабильная идентичность completion items

- Порядок элементов должен быть независим от `HashMap`-итерации (везде, где собираем из map/set → сортируем).
- `sortText` должен быть стабильным (и не меняться на `completionItem/resolve`, см. LSP spec).
- `candidate_id`: в P5 либо:
  - (A, рекомендовано) добавить минимальный стабильный `candidate_id` в `CompletionItem.data` (см. M6),
  - (B) зафиксировать, что `candidate_id` пока отсутствует (и тогда “стабильность” = отсутствие), но не мешать M6.

## TODO (P5)

### 0) Подготовка API v2 (чтобы фичи не читали глобальное состояние)

- [x] `analysis-v2`: добавить read API для deps data (например, `AnalysisV2::deps_data() -> Cancellable<Arc<SemanticDeps>>`).
- [ ] `analysis-v2`: (опционально) добавить компактный “observed context” хелпер для логов:
  `file_version + deps_id + settings_id`, чтобы унифицировать трассировку.
- [x] Зафиксировать правило: v2 путь берёт `file_text/file_path` из snapshot (а не из `documents`/диска), иначе риск mixed state.

### 1) Completion v2

- [x] Вынести v2 реализацию из `backend/src/bin/lsp_server/server/language_server.rs` в `backend/src/bin/lsp_server/handlers/completion.rs`:
  - [x] `handle_completion_v2(...)` принимает уже извлечённые данные (`file_content/file_path/ir_program/deps + Position + Url + index + snippet_support`)
    и возвращает `CompletionResponseWithStats` (без передачи `AnalysisV2` через `await`, чтобы избежать non-Send future).
  - [x] `language_server.rs` только роутит legacy/v2 по флагу и извлекает данные из снапшота до `await`.
- [x] В v2 ветке completion использовать:
  - [x] `analysis.file_text(file_id)` + `analysis.file_path(file_id)` как вход,
  - [x] `analysis.line_index(file_id)`/позиционирование через `analysis.utf16_position_to_*` (где нужно),
  - [x] `analysis.ir(file_id)` как единственный источник IR.
- [x] Переиспользовать `completion_service` алгоритм без legacy IR build:
  - [x] entrypoint `get_completion_with_semantic_program(...)` принимает `Arc<SemanticProgram>` + `resolver/repository` из deps snapshot (без `ParserCoordinator/IrCache`).
  - [x] v2 путь не вызывает `parse_to_ir`/`parse_with_cache_for_file` и не трогает `IrCache` (проверено `rg` по обработчикам).
- [x] Детерминизм completion результата:
  - [x] total ordering уже задан в `completion_ranking` (tie-breakers по source/label/kind/scope/owner),
  - [x] `origin_sources` стабилизированы (sort+dedup),
  - [x] `sortText`/порядок стабилен (smoke test: два запуска → одинаковый результат).
- [x] `candidate_id`: выбран вариант B — пока отсутствует (данные в `CompletionItem.data` содержат `kind/owner_type/origin_sources`); добавление стабильного `candidate_id` — в M6.

### 2) Hover v2

- [x] Аналогично completion: вынести v2 реализацию в `backend/src/bin/lsp_server/handlers/hover.rs` (или рядом),
  чтобы можно было покрыть тестами без поднятия всего сервера.
- [x] В v2 hover использовать:
  - [x] `analysis.file_text(file_id)` + `analysis.file_path(file_id)`,
  - [x] `analysis.ir(file_id)` как источник IR (без legacy `IrCache`),
  - [x] deps snapshot (`repository/resolver/signature_index`) через `deps_data()`.
- [x] Переиспользовать форматирование hover (существующие `HoverFormatter`/`format_semantic_node_info`) без IR build внутри.
- [x] Проверить критичный кейс: form modules (зависимость от `file_path`/`CodeLocation`) — `file_path` берём из snapshot и пробрасываем в IR query (AstToIrConverter тот же).
- [x] Детерминизм hover:
  - [x] smoke test: два запуска hover на одинаковом входе → одинаковый результат,
  - [x] лимиты (max_methods/max_properties) применяются через `HoverFormatConfig`.

### 3) SignatureHelp v2

- [x] Вынести v2 реализацию в `backend/src/bin/lsp_server/handlers/signature_help.rs`.
- [x] В v2 signatureHelp использовать:
  - [x] `analysis.file_text(file_id)` (а не `documents`/диск),
  - [x] deps snapshot (`repository/resolver`) из `deps_data()`,
  - [x] избегать `self.coordinator.get_analysis_engine()` в v2 пути (иначе риск mixed deps).
- [ ] (Опционально) улучшение качества: если receiver — переменная, а не тип, добавить best-effort вывод типа через IR
  (но не блокировать P5, если нужно быстрее закрыть миграцию).

### 4) Golden/fixture тесты и верификация

- [x] Добавить минимальный “двухрежимный” контур тестов (legacy vs v2) для:
  - [x] completion,
  - [x] hover,
  - [x] signatureHelp.
- [x] Реиспользовать существующие фикстуры/голдены:
  - `backend/src/bin/lsp_server/handlers/completion.rs` (`tests/fixtures`, `tests/golden`)
  - `backend/src/bin/lsp_server/handlers/signature_help.rs` (`tests/fixtures`, `tests/golden`)
  - для hover: добавлен dual-mode unit test (legacy vs v2) на fixture.
- [x] В тестах сравнения:
  - [x] прогонять оба пути и сравнивать результаты напрямую (при необходимости можно добавить нормализацию).
- [x] Добавить “smoke determinism” тесты:
  - [x] два запуска completion на одинаковом входе → одинаковый список (включая `sortText` и `data`),
  - [ ] (опционально) два `AnalysisHostV2` (две базы) на одном input → одинаковый результат.
- [x] Фактическая верификация “нет legacy IR build в v2 hot path”:
  - [x] `rg` по `parse_to_ir|parse_with_cache_for_file|IrCache` в v2 ветках обработчиков,
  - [x] логи observed ids показывают `deps_id/settings_id/file_version`.

## DoD (P5 считается закрытым, если)

- [x] При `BSL_INTELLISENSE_V2_SALSA=1` completion/hover/signatureHelp возвращают реальный результат (не заглушки) и не обращаются к legacy IR build.
- [x] Детерминизм: порядок + `sortText` (и выбранный вариант для `candidate_id`) стабильны.
- [x] Golden/fixture тесты для completion/hover/signatureHelp проходят в двух режимах и сравниваются.
- [x] `cargo test --workspace` проходит.
