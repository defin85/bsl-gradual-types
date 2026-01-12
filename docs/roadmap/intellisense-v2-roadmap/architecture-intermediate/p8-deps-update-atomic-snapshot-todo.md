# P8: TODO list — Интеграция deps_update (metadata/platform/index) как атомарный снапшот

**Дата:** 2026-01-10  
**Статус:** 🟢 DONE  
**Основание:** Фаза P8 из `docs/roadmap/intellisense-v2-roadmap/architecture-intermediate/salsa-migration-plan.md`

## Контекст (зачем P8)

В v2 (salsa) “deps” должны обновляться **атомарно**, иначе возможен класс ошибок “mixed deps”:

- `deps_id` говорит “зависимости уже новые”, а `deps_data` (repo/resolver/signature_index) ещё старые (или наоборот).
- индекс/метаданные/платформа меняются частями, и запрос (completion/hover/diagnostics) видит смесь.

Текущее состояние (после P8):

- `DepsBundleV2` (deps_id + semantic_deps + index_snapshot) строится целиком в `backend/src/system/deps_bundle_v2.rs` (`build_deps_bundle_v2`).
- `deps_update_v2` строит bundle в blocking и атомарно применяет его через writer thread:
  `backend/src/bin/lsp_server/server/core.rs` (`deps_update_v2`).
- v2 entrypoints получают согласованную пару `(AnalysisV2 snapshot, IndexSnapshot, deps_id)` через:
  `backend/src/bin/lsp_server/server/analysis_v2_runtime.rs` (`snapshot_with_deps`).

## Цель P8

- Ввести единый `DepsSnapshot` (metadata/platform/index) как **иммутабельный пакет**, который:
  - вычисляется целиком вне hot path,
  - применяется одним атомарным шагом (без частичных обновлений),
  - имеет стабильный `deps_id` (fingerprint), пригодный для логов/кэшей/диагностики.
- Обеспечить “no mixed deps” для v2:
  - IR/семантика зависят только от `deps_id` и получают deps только из снапшота.
  - completion/hover/signatureHelp используют **тот же** deps-пакет (включая индекс), что и salsa queries.
- Добавить наблюдаемость: сколько времени занимает build/apply deps_update и как часто он влияет на UX.

## Контракт (инварианты)

- **Atomic deps update:** запрос видит либо полностью “старые deps”, либо полностью “новые deps”, но не смесь.
- **No in-place deps mutation:** после публикации `DepsSnapshot` его содержимое не меняется “тихо” (иначе нарушается snapshot-safety).
- **No I/O in salsa queries:** всё, что читается с диска (metadata/platform/index), приходит только через deps snapshot.
- **Writer thread owns apply:** применение deps_update происходит через v2 writer thread
  (`backend/src/bin/lsp_server/server/analysis_v2_runtime.rs`), а не напрямую из async обработчиков.
- **Observability:** каждый ответ/публикация в v2 логирует `observed (file_version, deps_id, settings_id)` и (если участвует) `index_snapshot_id`.

## Внешние референсы (prior art)

- rust-analyzer: модель “apply changes + ask queries on snapshot”:
  - https://rust-analyzer.github.io/book/contributing/guide.html
  - https://rust-analyzer.github.io/book/contributing/architecture.html
- Salsa cancellation:
  - https://docs.rs/salsa/latest/salsa/enum.Cancelled.html

## Локальные референсы (в репо)

- v2 deps snapshot input: `analysis-v2/src/lib.rs` (`DepsSnapshot`, `Change::SetDepsSnapshot`, `SemanticDeps`).
- P7 writer thread runtime: `backend/src/bin/lsp_server/server/analysis_v2_runtime.rs`.
- Текущий v2 deps update: `backend/src/bin/lsp_server/server/core.rs` (`deps_update_v2`, `sync_v2_globals`).
- Команды, которые меняют metadata/подписи (потенциальный deps_update):
  - `backend/src/bin/lsp_server/server/command_handlers.rs` (`bsl/buildIndex`, `bsl/incrementalUpdate`)
  - `backend/src/bin/lsp_server/commands/configuration.rs` (`handle_parse_configuration`, `handle_incremental_update`)
- Platform/config загрузка в SystemCoordinator:
  - `backend/src/system/system_coordinator/lifecycle.rs` (platform + config startup)
  - `backend/src/system/system_coordinator/config_loader.rs` (fingerprints + индекс конфигурации)
- Индекс: `backend/src/system/intellisense_index.rs` (`IndexSnapshot`, `IntellisenseIndexStore`).

## Архитектурное решение (рекомендация)

### 1) Ввести единый “deps bundle” для v2

Добавляем (в backend/LSP слое) иммутабельную структуру (название условное):

- `DepsBundleV2`:
  - `deps_id: bsl_analysis_v2::DepsSnapshotId`
  - `semantic_deps: Arc<bsl_analysis_v2::SemanticDeps>` (repo/resolver/signature_index)
  - `index_snapshot: Arc<bsl_backend::system::IndexSnapshot>` (или эквивалентный read-only снапшот индекса)
  - (опционально) “отладочная мета”: `platform_version`, `config_fingerprint`, `index_snapshot_id`.

Важно: индекс не тащим внутрь `bsl-analysis-v2` (чтобы не ломать границы крейтов);
но обязуемся получать его **из того же bundle**, что и `semantic_deps`.

### 2) deps_update = build вне writer thread + atomic swap в writer thread

- Build (потенциально дорогой) выполняется в `spawn_blocking`/отдельном thread/pool:
  - собираем `SemanticDeps` (repo/resolver/signature_index) и `IndexSnapshot` в консистентном виде,
  - вычисляем новый `deps_id` из fingerprint-ов (см. ниже).
- Apply выполняется быстро в writer thread:
  - одним сообщением обновляем `Change::SetDepsSnapshot { deps_id, deps }`,
  - и синхронно обновляем `index_snapshot` (в состоянии runtime), чтобы `snapshot_with_deps()` возвращал согласованную пару.

### 3) deps_id: что должно входить

`deps_id` должен меняться при любых изменениях, влияющих на семантику/индекс:

- платформа (platform types / platform version / syntax helper fingerprint),
- метаданные конфигурации + индекс экспортов (XML + BSL modules, желательно layer B),
- версия схемы индекса/алгоритмов (schema/settings fingerprint).

Минимально допустимо (если нужно быстро закрыть P8):
- `deps_id = hash(schema + platform_version + config_layer_b_fingerprint + strict_mode + index_schema_version)`.

Важно: `deps_id` должен быть частью `DepsBundleV2`, а не вычисляться “на лету” от разрозненных глобальных источников.

### 4) Как запросы получают индекс (без mixed)

Рекомендация: расширить runtime API:

- `AnalysisV2Runtime::snapshot_with_deps() -> (AnalysisV2, Arc<IndexSnapshot>, DepsSnapshotId)`

и использовать это во всех v2 entrypoints (completion/hover/signatureHelp/diagnostics),
чтобы индекс и `deps_id` всегда соответствовали снапшоту.

## TODO list (реализация)

### 0) API и типы “deps bundle”

- [x] Ввести `DepsBundleV2` (backend/LSP слой): `deps_id + semantic_deps + index_snapshot (+ meta)`.
- [x] Зафиксировать функцию/модуль для вычисления `deps_id` из fingerprint-ов.
- [x] Определить явную границу “что входит в deps” (metadata/platform/index) и что остаётся вне.

### 1) Build pipeline: собрать новый deps snapshot целиком

- [x] Реализовать builder `build_deps_bundle_v2(...)`:
  - [x] собирает `semantic_deps` (repo/resolver/signature_index) в консистентном виде,
  - [x] собирает `index_snapshot` (read-only) в консистентном виде,
  - [x] вычисляет `deps_id`.
- [x] Гарантировать отсутствие частичных апдейтов:
  - [x] выбран подход: строго ограничиваем момент публикации (старый bundle остаётся активен до конца build).
  - (альтернатива, не делаем) строить новый репозиторий/индекс “с нуля” и затем делать swap.

### 2) Apply pipeline: атомарно применить bundle через writer thread

- [x] Расширить `backend/src/bin/lsp_server/server/analysis_v2_runtime.rs`:
  - [x] хранить текущий `index_snapshot` в состоянии writer thread,
  - [x] добавить команду “ApplyDepsBundle” (deps + index) или эквивалентный протокол,
  - [x] добавить `snapshot_with_deps()`.
- [x] Обеспечить корректное поведение при shutdown (release waiters, как в P7).

### 3) Интеграция с реальными deps_update источниками

- [x] После `SystemCoordinator::start_with_paths(...)` (platform+config загрузка):
  - [x] построить новый `DepsBundleV2`,
  - [x] применить в runtime (v2).
- [x] После `bsl/buildIndex` / `bsl/parseConfiguration`:
  - [x] строить `DepsBundleV2` из результата парсинга/индексации,
  - [x] атомарно применять в runtime.
- [x] После `bsl/incrementalUpdate`:
  - [x] строить новый `DepsBundleV2` (полный или инкрементальный),
  - [x] атомарно применять в runtime.

### 4) Перевод v2 фич на “deps bundle” (без чтения глобального mutable index)

- [x] completion/hover/signatureHelp v2:
  - [x] получать `index_snapshot` из `snapshot_with_deps()`, а не из `SystemCoordinator::intellisense_index()`.
- [x] diagnostics v2:
  - [x] продолжать freshness gate по `(file_version, deps_id, settings_id)`,
  - [x] добавить логирование `index_snapshot_id` (если влияет на публикацию/качество).

### 5) Observability

- [x] Метрики:
  - [x] latency build deps bundle,
  - [x] latency apply deps bundle (очередь writer thread),
  - [x] счетчик deps_update (успех/ошибка).
- [x] Логи:
  - [x] при применении deps_update логировать `deps_id` и ключевые fingerprint-ы.

### 6) Тесты

- [x] Unit tests:
  - [x] `analysis_v2_runtime` умеет атомарно менять deps + index и отдавать согласованную пару.
- [x] Интеграционный тест:
  - [x] deps_update во время редактирования/запросов не приводит к mixed deps
    (результат относится либо к старому, либо к новому `deps_id`).
- [x] `cargo test --workspace` проходит.

## DoD (P8 считается закрытым, если)

- [x] В v2 есть явный `deps_update` путь: build new snapshot -> atomic apply (через writer thread).
- [x] `deps_id` меняется при изменениях metadata/platform/index и больше не “привязан” только к `IndexSnapshotId`.
- [x] completion/hover/signatureHelp/diagnostics v2 используют один согласованный deps bundle (без mixed).
- [x] Есть тест, который ловит regressions “deps_update во время редактирования”.
- [x] `cargo test --workspace` проходит.

## Ручная проверка (рекомендовано)

- [ ] Сценарий: во время `bsl/buildIndex` или `bsl/incrementalUpdate` активно печатать и вызывать completion/hover:
  - нет подвисаний/дедлоков,
  - результаты соответствуют одному `deps_id` на запрос,
  - после завершения обновления появляются данные из новых метаданных/индекса.

## Верификация (факты)

- ✅ `backend/src/system/deps_bundle_v2.rs`: `DepsBundleV2`, `DepsBundleV2Meta`, `build_deps_bundle_v2` (deps_id + semantic_deps + index_snapshot).
- ✅ `backend/src/bin/lsp_server/server/analysis_v2_runtime.rs`: `ApplyDepsBundle`, `snapshot_with_deps`, `apply_deps_bundle`; unit test `p8_snapshot_with_deps_is_atomic`.
- ✅ `backend/src/bin/lsp_server/server/core.rs`: `deps_update_v2` (build+apply+metrics), diagnostics v2 использует `snapshot_with_deps`; integration test `p8_deps_update_is_atomic_and_completion_uses_runtime_index_snapshot`.
- ✅ `backend/src/bin/lsp_server/server/language_server.rs`: v2 completion/hover/signatureHelp используют `snapshot_with_deps`; `bsl.parseConfiguration` вызывает `deps_update_v2` после успеха.
- ✅ `backend/src/bin/lsp_server/server/command_handlers.rs`: `bsl/buildIndex` и `bsl/incrementalUpdate` вызывают `deps_update_v2` после успеха.
- ✅ `backend/src/system/basic_observability.rs` и `backend/src/system/system_coordinator/coordinator.rs`: метрики deps_update build/apply latency + success/error.
- ✅ `cargo test -p bsl-backend --bin bsl-lsp-server p8_` — OK (2/2).
- ✅ `cargo test -p bsl-backend --bin bsl-lsp-server` — OK.
- ✅ `cargo test --workspace` — OK.
