# P9: TODO list — Удаление legacy путей и кэшей

**Дата:** 2026-01-10  
**Актуализировано:** 2026-01-11  
**Статус:** 🟠 В процессе (частично выполнено)  
**Основание:** Фаза P9 из `docs/roadmap/intellisense-v2-roadmap/architecture-intermediate/salsa-migration-plan.md`

## Analysis

После P5–P8 в репозитории одновременно существуют два пути:

- **v2 (salsa / rust-analyzer style)**: `AnalysisHostV2` + snapshots + writer thread + атомарный deps bundle.
- **legacy**: `TypeSystemService` + `ParserCoordinator` + `AnalysisCache`/`IrCache` + LSP handlers, которые делают часть работы “на лету”.

Это даёт несколько проблем:

- **Дублирование логики и расхождение поведения** (фикс делается в одном пути, регресс появляется в другом).
- **Скрытая “модель истинности”**: часть истины о состоянии документа живёт в `documents`/кэшах, часть — в v2 inputs.
- **Риск возврата mixed state/mixed deps** через legacy ветки и global mutable cache-и.
- **Сложность поддержки**: feature-flag + ветвления в LSP усложняют дебаг и тестирование.

Архитектурный драйвер P9: после стабилизации сделать **v2 единственным путём вычислений** для LSP/CLI/Web и затем удалить legacy (ветки, сервисы, кэши).

Дополнительная цель P9: не просто “удалить ветки”, а **унифицировать резолвинг типов** вокруг v2,
то есть сделать v2 (salsa inputs/queries + deps snapshot) единственным источником:
- `SemanticProgram` (IR) для intellisense,
- `SemanticDeps` (repo/resolver/signature_index) для type resolution/lookup,
- observed контекста (`file_version/deps_id/settings_id`) для публикации результатов.

Иначе мы рискуем “переименовать” legacy в другое место: LSP станет v2-ом, но CLI/Web и часть доменной логики
останутся на `TypeSystemService` + `ParserCoordinator` + кэшах, и будут продолжать расходиться по поведению/корректности.

Целевое состояние P9: **весь необходимый legacy‑функционал переехал в v2**, после чего legacy‑код удалён из репозитория.

## Recommendations

Рекомендация для P9 (в порядке предпочтения):

1) **LSP = только v2**:
   - убрать runtime feature flag `BSL_INTELLISENSE_V2_SALSA`,
   - удалить legacy ветки в LSP обработчиках,
   - добить оставшиеся LSP entrypoints, которые ещё используют `TypeSystemService` (например, `textDocument/definition`).
2) **Legacy кэши (`IrCache`/`AnalysisCache`)**:
   - мигрировать всех клиентов, которые их используют, на v2 (queries + deps snapshot),
   - после миграции удалить кэши (не оставлять “только для CLI/Web” как второй путь).
3) **TypeSystemService**:
   - **решение:** перевести клиентов на v2‑entrypoints (или тонкий фасад над `AnalysisV2 + DepsBundleV2`),
     после чего удалить `TypeSystemService` и связанный legacy код.
   - LSP/CLI/Web должны использовать один и тот же слой резолвинга (v2) и один observed контекст (`file_version/deps_id/settings_id`).
4) **Единый слой резолвинга поверх v2**:
   - закрепить v2 как “канонический” источник IR+deps для всех фич (completion/hover/signatureHelp/definition/diagnostics),
   - держать алгоритмы вычисления результата в одном месте (общие entrypoints), чтобы LSP/CLI/Web не расходились.

## Implementation Considerations

- Удаление legacy путей лучше делать как “strangler” завершение: сначала убедиться, что v2 покрывает все нужные LSP endpoints,
  затем удалить флаг и код (git history остаётся страховкой).
- При удалении веток следить за **границами ответственности**:
  - `bsl-analysis-v2` не должен тащить `bsl-backend`,
  - LSP слой не должен зависеть от legacy кэшей/сервисов.
- Унификация резолвинга не означает “тащить LSP в salsa”: `bsl-analysis-v2` остаётся слоем `inputs → queries`
  (текст/позиционирование/parse_result/IR/diagnostics), а “бизнес-логика” intellisense остаётся в backend (или выносится в отдельный crate),
  но работает **только** на данных из v2 (IR + deps snapshot + index snapshot).
- До удаления кода важно подготовить **механические проверки** (rg/CI), чтобы предотвращать возврат `parse_to_ir` в hot path.

## Risks

- Возможна деградация функциональности, если часть LSP фич всё ещё реализована только в legacy (пример: `goto_definition`).
- Удаление кэшей может неожиданно ударить по CLI/Web сценариям (если они реально используют `AnalysisCache`/`IrCache`).
- После удаления fallback усложняется быстрый rollback: нужен либо релизный флаг на уровне версии, либо быстрый hotfix.

## Внешние референсы (prior art)

- Strangler Fig Application (инкрементальное вытеснение legacy):
  - https://martinfowler.com/bliki/StranglerFigApplication.html
- Feature Toggles / Feature Flags (как держать миграцию под контролем и не забыть удалить флаг):
  - https://martinfowler.com/articles/feature-toggles.html

## Локальные референсы (в репо)

- v2 инфраструктура:
  - `analysis-v2/src/lib.rs` (`AnalysisHostV2`, `AnalysisV2`, inputs/queries).
  - `backend/src/bin/lsp_server/server/analysis_v2_runtime.rs` (writer thread + snapshots + `snapshot_with_deps`).
  - `backend/src/bin/lsp_server/server/deps_v2.rs` (`DepsBundleV2`, `build_deps_bundle_v2`).
- Legacy LSP путь (кандидаты на удаление/миграцию):
  - `backend/src/bin/lsp_server/handlers/*` (completion/hover/definition/signature_help/text_document).
  - `backend/src/application/type_system/service.rs` (`TypeSystemService`).
  - `backend/src/system/ir_cache.rs`, `backend/src/system/simple_cache.rs`.
- Feature flag:
  - `backend/src/bin/lsp_server/server/core.rs`, `backend/src/bin/lsp_server/server/mod.rs`,
    `backend/src/bin/lsp_server/server/language_server.rs` (`BSL_INTELLISENSE_V2_SALSA`, `use_salsa_v2`).

## TODO list (реализация)

### 0) Инвентаризация “legacy” и границы удаления

- [x] Зафиксировать “кто клиенты legacy”: LSP / Web API / CLI (что именно использует `TypeSystemService`, `IrCache`, `AnalysisCache`).
- [x] Составить список entrypoints (LSP/Web/CLI), которые ещё используют legacy:
  - [x] `rg -n "TypeSystemService" backend/src -S`
  - [x] `rg -n "parse_to_ir" backend/src -S`
  - [x] `rg -n "IrCache|AnalysisCache" backend/src -S`

### 1) Перевести `goto_definition` на v2 (и добить остальные LSP endpoints)

- [x] `textDocument/definition` (`goto_definition`): реализовать v2 путь:
  - [x] вход: `(FileId, file_version, position)` + `(AnalysisV2 snapshot, deps bundle)`,
  - [x] получать IR/семантику только из v2 queries,
  - [x] сохранить текущую семантику разрешения (конфигурационные типы / user-defined / platform).
- [ ] (Если остаются) другие endpoints, которые зовут `TypeSystemService` — мигрировать/убрать.

### 2) Удалить feature flag `BSL_INTELLISENSE_V2_SALSA` и ветвления в LSP

- [x] `backend/src/bin/lsp_server/server/core.rs`: убрать чтение env `BSL_INTELLISENSE_V2_SALSA`.
- [x] `backend/src/bin/lsp_server/server/mod.rs`: удалить поле `use_salsa_v2`.
- [x] `backend/src/bin/lsp_server/server/language_server.rs`: удалить legacy ветки `if self.use_salsa_v2 { ... } else { ... }`.
- [ ] (Если нужно) временно заменить runtime flag на compile-time feature `legacy-lsp` (чтобы проще чистить код), затем удалить и его (не понадобилось).

### 3) Удалить legacy LSP handlers и проводку

- [ ] Удалить `backend/src/bin/lsp_server/handlers/*` (или оставить только то, что ещё нужно и не относится к LSP).
- [ ] Удалить хранение “истины текста” в `documents` (если после P9 оно больше не нужно):
  - [ ] заменить все чтения текста на чтение из v2 (`AnalysisV2::file_text`) или на локальный “request copy”.

### 4) Убрать legacy кэши из LSP и решить судьбу кэшей глобально

- [ ] LSP server не должен:
  - [ ] держать `IrCache`/`AnalysisCache`,
  - [ ] чистить `IrCache` при загрузке platform types,
  - [x] использовать `ParserCoordinator::parse_to_ir` в запросах.
- [ ] Решение по `IrCache`/`AnalysisCache`:
  - [ ] мигрировать всех клиентов на v2,
  - [ ] удалить `IrCache`/`AnalysisCache` (не оставлять “только для CLI/Web”).

### 5) Унифицировать резолвинг типов через v2 (не только LSP)

Цель: одна “каноническая” модель вычислений (v2) и единые entrypoints алгоритмов, чтобы:
- LSP/CLI/Web давали совместимые результаты на одном и том же deps snapshot,
- legacy кэши стали необязательными и могли быть удалены без потери функциональности.

- [ ] Зафиксировать границу “что такое v2-резолвинг”:
  - [ ] IR берётся только из `AnalysisV2::ir(file_id)` (salsa query),
  - [ ] deps/resolver/signature_index берутся только из `DepsSnapshot` / `SemanticDeps` (P8 deps bundle),
  - [ ] любые дополнительные данные (например, индекс) должны приходить как snapshot рядом с deps.
- [ ] Вынести общий слой вычисления результата (без привязки к LSP) в единый модуль:
  - [ ] completion: один entrypoint, который принимает `(text, position, ir_program, deps, index_snapshot)`,
  - [ ] hover: один entrypoint, который принимает `(text, position, ir_program, deps, settings)`,
  - [ ] signatureHelp: один entrypoint, который принимает `(text, position, deps)`,
  - [ ] goto_definition: один entrypoint, который принимает `(text, position, ir_program, deps, maybe_paths/index)`.
- [ ] Перевести CLI/Web на этот же слой:
  - [ ] для “анализ текста”/“hover по фрагменту”/“семантика по файлу” получать IR через v2 (возможен отдельный
        `AnalysisHostV2` per-request или долгоживущий host в coordinator),
  - [ ] постепенно убрать прямые вызовы legacy веток, где внутри строится IR через `ParserCoordinator::parse_to_ir`.
- [ ] Принять решение по судьбе `TypeSystemService`:
  - [x] решение: весь функционал `TypeSystemService` (нужный LSP/CLI/Web) переезжает на v2;
        `TypeSystemService` может временно стать thin‑фасадом над v2, но целевое состояние P9 — удалить `TypeSystemService`.
  - [ ] перевести CLI/Web на v2‑entrypoints (без `ParserCoordinator::parse_to_ir` и без legacy кэшей).
  - [ ] после миграции удалить `TypeSystemService` и связанные legacy модули/кэши.
- [ ] После миграции CLI/Web: удалить `IrCache`/`AnalysisCache` (оптимизации — только snapshot-safe внутри v2).

### 6) Документация, миграционные заметки, наблюдаемость

- [ ] Удалить/обновить упоминания `BSL_INTELLISENSE_V2_SALSA` в документации (если есть).
- [ ] Зафиксировать в docs: “LSP v2 не использует legacy caches”.
- [ ] Проверить метрики/логи: после P9 они должны отражать только v2 путь (без дублирования).

## DoD (P9 считается закрытым, если)

- [ ] В LSP коде отсутствует ветвление `use_salsa_v2` и env `BSL_INTELLISENSE_V2_SALSA`.
- [ ] В `backend/src/bin/lsp_server` нет зависимостей от `TypeSystemService`, `IrCache`, `AnalysisCache`.
- [ ] `parse_to_ir` не используется из LSP hot path (completion/hover/signatureHelp/definition).
- [ ] CLI/Web entrypoints для резолвинга типов/intellisense используют v2 и не зависят от legacy (`TypeSystemService`, `IrCache`, `AnalysisCache`, `ParserCoordinator::parse_to_ir`).
- [ ] `TypeSystemService`, `IrCache`, `AnalysisCache` удалены из `backend/src` после миграции всех клиентов.
- [ ] `cargo test -p bsl-backend --bin bsl-lsp-server` проходит.

## Верификация (факты)

### Актуально на 2026-01-11 (рабочая копия)

- `cargo test -p bsl-backend --bin bsl-lsp-server`:
  ```text
  test result: ok. 20 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.31s
  ```
- Feature flag / ветвления в LSP:
  - `rg -l "BSL_INTELLISENSE_V2_SALSA" backend/src -S` -> (пусто)
  - `rg -l "use_salsa_v2" backend/src/bin/lsp_server -S` -> (пусто)
- `parse_to_ir` в LSP hot path:
  - `rg -l "parse_to_ir" backend/src/bin/lsp_server -S` -> (пусто)
  - но в `backend/src` есть usage:
    ```text
    backend/src/application/type_system/services/completion_service.rs
    backend/src/application/type_system/services/file_analysis_service.rs
    backend/src/system/parser_coordinator.rs
    ```
- Legacy в LSP (пока не выполнен DoD про отсутствие `TypeSystemService`/кэшей):
  - `rg -l "TypeSystemService" backend/src/bin/lsp_server -S`:
    ```text
    backend/src/bin/lsp_server/commands/semantic.rs
    backend/src/bin/lsp_server/handlers/completion.rs
    backend/src/bin/lsp_server/handlers/definition.rs
    backend/src/bin/lsp_server/handlers/hover.rs
    backend/src/bin/lsp_server/handlers/text_document.rs
    backend/src/bin/lsp_server/server/core.rs
    ```
  - `rg -l "IrCache|AnalysisCache" backend/src/bin/lsp_server -S`:
    ```text
    backend/src/bin/lsp_server/handlers/completion.rs
    backend/src/bin/lsp_server/handlers/hover.rs
    ```
  - Также остаётся `coordinator.ir_cache().clear().await` в `backend/src/bin/lsp_server/server/language_server.rs` (legacy кэш в LSP).
- Legacy в `backend/src` (CLI/Web/координатор пока на legacy пути):
  - `rg -l "TypeSystemService" backend/src -S`:
    ```text
    backend/src/README.md
    backend/src/application/README.md
    backend/src/application/mod.rs
    backend/src/application/type_system/extractors/mod.rs
    backend/src/application/type_system/loaders/configuration_loader.rs
    backend/src/application/type_system/mod.rs
    backend/src/application/type_system/service.rs
    backend/src/application/type_system/services/mod.rs
    backend/src/bin/intellisense_perf.rs
    backend/src/bin/lsp_server/commands/semantic.rs
    backend/src/bin/lsp_server/handlers/completion.rs
    backend/src/bin/lsp_server/handlers/definition.rs
    backend/src/bin/lsp_server/handlers/hover.rs
    backend/src/bin/lsp_server/handlers/text_document.rs
    backend/src/bin/lsp_server/server/core.rs
    backend/src/helpers/hover_formatter/mod.rs
    backend/src/lib.rs
    backend/src/main.rs
    backend/src/presentation/semantic_routes.rs
    backend/src/presentation/web/handlers.rs
    backend/src/system/system_coordinator/coordinator.rs
    backend/src/system/system_coordinator/lifecycle.rs
    ```
  - `rg -l "IrCache|AnalysisCache" backend/src -S`:
    ```text
    backend/src/README.md
    backend/src/application/type_system/service.rs
    backend/src/application/type_system/services/completion_service.rs
    backend/src/application/type_system/services/file_analysis_service.rs
    backend/src/application/type_system/services/hover_service.rs
    backend/src/application/type_system/services/web_api_service.rs
    backend/src/bin/lsp_server/handlers/completion.rs
    backend/src/bin/lsp_server/handlers/hover.rs
    backend/src/system/ir_cache.rs
    backend/src/system/mod.rs
    backend/src/system/simple_cache.rs
    backend/src/system/system_coordinator/coordinator.rs
    backend/src/system/system_coordinator/types.rs
    ```
- Warnings (dead code) при сборке `bsl-lsp-server` (на фоне v2-only веток в `language_server.rs`):
  ```text
  warning: function `handle_goto_definition` is never used
  warning: function `handle_did_open` is never used
  warning: function `handle_did_change` is never used
  ```

### Заполнить при завершении P9

- `rg` проверки на отсутствие legacy в LSP.
- `rg` проверки на отсутствие legacy в CLI/Web.
- Ссылки на конкретные файлы/коммиты, где удалены ветки и кэши.
- Вывод тестов (минимум): `cargo test -p bsl-backend --bin bsl-lsp-server`.
