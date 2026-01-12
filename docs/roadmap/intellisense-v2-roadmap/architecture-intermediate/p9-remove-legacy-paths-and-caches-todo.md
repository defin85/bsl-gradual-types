# P9: TODO list — Удаление legacy путей и кэшей

**Дата:** 2026-01-10  
**Актуализировано:** 2026-01-12  
**Статус:** 🟢 Выполнено  
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
  - `backend/src/system/deps_bundle_v2.rs` (`DepsBundleV2`, `build_deps_bundle_v2`).
- Legacy LSP путь (кандидаты на удаление/миграцию):
  - `backend/src/bin/lsp_server/handlers/*` (completion/hover/definition/signature_help/text_document).
  - `backend/src/application/type_system/service.rs` (`TypeSystemService`) — удалено в P9.
  - `backend/src/system/ir_cache.rs`, `backend/src/system/simple_cache.rs` — удалены в P9.
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
- [x] Другие endpoints, которые звали `TypeSystemService`, удалены/мигрированы (v2-only).

### 2) Удалить feature flag `BSL_INTELLISENSE_V2_SALSA` и ветвления в LSP

- [x] `backend/src/bin/lsp_server/server/core.rs`: убрать чтение env `BSL_INTELLISENSE_V2_SALSA`.
- [x] `backend/src/bin/lsp_server/server/mod.rs`: удалить поле `use_salsa_v2`.
- [x] `backend/src/bin/lsp_server/server/language_server.rs`: удалить legacy ветки `if self.use_salsa_v2 { ... } else { ... }`.
- [x] (Не понадобилось) временно заменять runtime flag на compile-time feature `legacy-lsp`.

### 3) Удалить legacy LSP handlers и проводку

- [x] Удалить legacy ветки внутри `backend/src/bin/lsp_server/handlers/*` (handlers остаются, но путь вычислений только v2).
- [x] Source of truth текста для анализа — v2 inputs (нет параллельного хранения текста “для анализа” в LSP).

### 4) Убрать legacy кэши из LSP и решить судьбу кэшей глобально

- [x] LSP server не держит legacy кэши:
  - [x] `IrCache`/`AnalysisCache` отсутствуют,
  - [x] нет очистки `IrCache` при загрузке platform types,
  - [x] нет `ParserCoordinator::parse_to_ir` в LSP запросах.
- [x] `IrCache`/`AnalysisCache`: клиенты мигрированы на v2, кэши удалены.

### 5) Унифицировать резолвинг типов через v2 (не только LSP)

Цель: одна “каноническая” модель вычислений (v2) и единые entrypoints алгоритмов, чтобы:
- LSP/CLI/Web давали совместимые результаты на одном и том же deps snapshot,
- legacy кэши стали необязательными и могли быть удалены без потери функциональности.

- [x] Граница “v2-резолвинга” зафиксирована на практике:
  - [x] IR берётся из `AnalysisV2::ir(file_id)` (salsa query),
  - [x] deps/resolver/signature_index берутся из `DepsSnapshot` / `SemanticDeps` (deps bundle),
  - [x] индекс приходит как `IndexSnapshot` рядом с deps.
- [x] Общие entrypoints вынесены в `backend/src/application/type_system/services/*` (без привязки к LSP):
  - [x] completion: `get_completion_with_semantic_program_snapshot(...)`,
  - [x] hover: `get_hover_info_with_semantic_program(...)`,
  - [x] signatureHelp/goto_definition: v2-only путь в LSP (без legacy сервисов/кэшей).
- [x] CLI/Web используют v2 deps snapshot (и, где нужно, v2 IR) без legacy сервисов/кэшей.
- [x] `TypeSystemService` удалён; legacy модули и кэши удалены.

### 6) Документация, миграционные заметки, наблюдаемость

- [x] Обновить упоминания `BSL_INTELLISENSE_V2_SALSA` в документации (P9 удаляет флаг).
- [x] Зафиксировать в docs: “LSP v2 не использует legacy caches”.
- [x] Метрики/логи: в LSP остаётся только v2 путь (без дублирования legacy).

## DoD (P9 считается закрытым, если)

- [x] В LSP коде отсутствует ветвление `use_salsa_v2` и env `BSL_INTELLISENSE_V2_SALSA`.
- [x] В `backend/src/bin/lsp_server` нет зависимостей от `TypeSystemService`, `IrCache`, `AnalysisCache`.
- [x] `parse_to_ir` не используется из LSP hot path (completion/hover/signatureHelp/definition).
- [x] CLI/Web entrypoints для резолвинга типов/intellisense используют v2 и не зависят от legacy (`TypeSystemService`, `IrCache`, `AnalysisCache`, `ParserCoordinator::parse_to_ir`).
- [x] `TypeSystemService`, `IrCache`, `AnalysisCache` удалены из `backend/src` после миграции всех клиентов.
- [x] `cargo test -p bsl-backend --bin bsl-lsp-server` проходит.

## Верификация (факты)

### Актуально на 2026-01-12 (рабочая копия)

- `cargo test -p bsl-backend --bin bsl-lsp-server -- --color never`:
  ```text
  test result: ok. 20 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.31s
  ```
- Feature flag / ветвления в LSP:
  - `rg -l "BSL_INTELLISENSE_V2_SALSA" backend/src -S` -> (пусто)
  - `rg -l "use_salsa_v2" backend/src/bin/lsp_server -S` -> (пусто)
- `parse_to_ir` в LSP hot path:
  - `rg -l "parse_to_ir" backend/src/bin/lsp_server -S` -> (пусто)
- Legacy в LSP и `backend/src`:
  - `rg -l "TypeSystemService" backend/src/bin/lsp_server -S` -> (пусто)
  - `rg -l "IrCache|AnalysisCache" backend/src/bin/lsp_server -S` -> (пусто)
  - `rg -l "TypeSystemService|IrCache|AnalysisCache" backend/src -S` -> (пусто)
