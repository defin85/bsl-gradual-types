# Components Detailed

Текущее описание production-компонентов BSL Gradual Type System после canonical semantic cutover.

Этот документ сознательно не повторяет milestone-era architecture с AST-based `TypeSystemFacade`
и legacy fallback paths. Исторические детали остались в git history; ниже только то, что реально
участвует в shipped runtime.

## Обзор слоёв

```text
SystemCoordinator / DepsBundleV2
  -> AnalysisHostV2 / AnalysisV2
  -> SemanticProgram (canonical IR + semantic_facts)
  -> derived exact semantic index
  -> IntellisenseV2Facade runtime
  -> shared type-system services
  -> LSP / Web / MCP / CLI adapters
```

## 1. System Layer

### SystemCoordinator

**Назначение:** composition root и владелец shared runtime dependencies.

Ключевые зоны:

- startup и загрузка конфигурации/платформенных данных;
- observability;
- index storage;
- shared runtime facade для semantic операций.

Основные файлы:

- [backend/src/system/mod.rs](../../backend/src/system/mod.rs)
- [bsl-runtime/src/system/system_coordinator/coordinator/observability.rs](../../bsl-runtime/src/system/system_coordinator/coordinator/observability.rs)

### DepsBundleV2

**Назначение:** immutable bundle semantic dependencies для конкретного snapshot.

Содержит:

- repository;
- resolver;
- signature index;
- discovery/search index (`IndexSnapshot`);
- `deps_id`, который участвует в exact artifact key.

Файл:

- [bsl-runtime/src/system/deps_bundle_v2.rs](../../bsl-runtime/src/system/deps_bundle_v2.rs)

## 2. Canonical Semantic Core

### AnalysisHostV2 / AnalysisV2

**Назначение:** revision-bound analysis graph и public query surface над ним.

Что важно сейчас:

- `ir(...)` строит canonical semantic snapshot;
- exact type lookup и `serve_only` доступны только для current revision;
- stale artifact не обслуживает semantic query;
- flow-sensitive overlay строится поверх canonical base result.

Основные файлы:

- [analysis-v2/src/lib/analysis_api.rs](../../analysis-v2/src/lib/analysis_api.rs)
- [analysis-v2/src/lib/snapshots.rs](../../analysis-v2/src/lib/snapshots.rs)

### SemanticProgram

**Назначение:** canonical IR snapshot конкретной revision.

Содержит:

- semantic nodes;
- symbol table;
- CFG;
- `semantic_facts`.

Файлы:

- [shared/src/ir/program.rs](../../shared/src/ir/program.rs)
- [shared/src/ir/mod.rs](../../shared/src/ir/mod.rs)

### SemanticFacts

**Назначение:** canonical semantic payload, из которого materialize-ится derived exact semantic index.

После cutover хранит:

- type facts по span/offset;
- receiver/member hints для member access и calls;
- definition anchors;
- callable/signature facts;
- additional recovery materialization для incomplete member access.

Файл:

- [shared/src/ir/semantic_facts.rs](../../shared/src/ir/semantic_facts.rs)

### type_inference_v2

**Назначение:** materialize canonical semantic facts из parse/IR topology.

Критичные обязанности:

- typed bindings для module context (`ЭтотОбъект`, `Объект`);
- facet-aware `TypeResolution`;
- recovery support для incomplete member access;
- serialization definition anchors, включая configuration XML path.

Файлы:

- [analysis-v2/src/type_inference_v2.rs](../../analysis-v2/src/type_inference_v2.rs)
- [analysis-v2/src/type_inference_v2/local_function_summaries.rs](../../analysis-v2/src/type_inference_v2/local_function_summaries.rs)

## 3. Exact Semantic Artifact

Этот слой часто исторически называется `type_index`/`serve_only`, но теперь это не отдельная truth,
а exact current-revision projection того же canonical IR snapshot.

Он используется для:

- fast `type-at-position`;
- owner hints;
- exact readiness checks;
- fail-closed serve reason profiling.

Правила:

- строится только из текущего IR snapshot;
- не делает повторный inference из `parse_result.program`;
- не использует stale artifact как substitute;
- не является discovery/search index.

Основные файлы:

- [analysis-v2/src/lib/snapshots.rs](../../analysis-v2/src/lib/snapshots.rs)
- [analysis-v2/src/lib/analysis_api.rs](../../analysis-v2/src/lib/analysis_api.rs)

## 4. Shared Runtime Facade

### IntellisenseV2Facade

**Назначение:** единый runtime contract для LSP/Web/MCP/CLI.

Отвечает за:

- preparation exact operation snapshot;
- queue priority и bounded wait;
- runtime observability stages;
- fail-closed semantics на adapter boundary;
- dispatch в shared semantic services.

Файлы:

- [bsl-runtime/src/application/intellisense_v2/facade.rs](../../bsl-runtime/src/application/intellisense_v2/facade.rs)
- [bsl-runtime/src/application/intellisense_v2/facade/runtime.rs](../../bsl-runtime/src/application/intellisense_v2/facade/runtime.rs)
- [bsl-runtime/src/application/intellisense_v2/policy.rs](../../bsl-runtime/src/application/intellisense_v2/policy.rs)

### Queue priority

Фактический runtime priority сейчас такой:

- `interactive`: `completion`, `hover`, `signatureHelp`, `definition`
- `background`: `diagnostics`, `members`, `type_at_position`, `symbol_search`, `references`, и др.

Это важно для docs/perf interpretation: не все semantic операции считаются interactive в scheduler,
но все они обязаны читать один и тот же canonical semantic contract.

## 5. Shared Semantic Services

### Completion Service

**Назначение:** semantic completion без adapter-local truth.

Текущий contract:

- syntax helpers извлекают только позицию/receiver slice;
- owner/member truth приходит из shared exact hints текущей revision;
- non-member semantic path не использует discovery/search `IndexSnapshot`;
- при miss current-revision exact artifact completion работает fail-closed.

Файлы:

- [bsl-runtime/src/application/type_system/services/completion_service.rs](../../bsl-runtime/src/application/type_system/services/completion_service.rs)
- [bsl-runtime/src/application/type_system/services/completion_service/context.rs](../../bsl-runtime/src/application/type_system/services/completion_service/context.rs)
- [bsl-runtime/src/application/type_system/services/completion_service/member_resolution.rs](../../bsl-runtime/src/application/type_system/services/completion_service/member_resolution.rs)
- [bsl-runtime/src/application/type_system/services/completion_service/scope_candidates.rs](../../bsl-runtime/src/application/type_system/services/completion_service/scope_candidates.rs)

### Hover Service

**Назначение:** user-facing type/member hover поверх canonical facts.

Analyzed path читает exact type/member facts из derived exact semantic index и использует canonical IR
только для node lookup/formatting context, а не для request-time rescue из consumer-local repository state.

Файл:

- [bsl-runtime/src/application/type_system/services/hover_service.rs](../../bsl-runtime/src/application/type_system/services/hover_service.rs)

### Definition Service

**Назначение:** canonical go-to-definition.

После закрытия `xgg.3` сервис:

- читает definition anchors, receiver types и method targets из derived exact semantic index;
- использует canonical `TypeDefinitionLocation` для configuration/common-module/local targets;
- не восстанавливает configuration XML path локально из consumer repo.

Файл:

- [bsl-runtime/src/application/type_system/services/definition_service.rs](../../bsl-runtime/src/application/type_system/services/definition_service.rs)

### Signature Help Service

**Назначение:** callable/signature lookup для LSP.

На analyzed path сигнатура берётся из callable targets exact semantic index, materialized из того же IR snapshot,
а не из request-time repository rescue.

Файл:

- [bsl-runtime/src/application/type_system/services/signature_help_service.rs](../../bsl-runtime/src/application/type_system/services/signature_help_service.rs)

## 6. Metadata and Repository Layer

### TypeRepository / TypeMetadataLookup / SignatureIndex

**Назначение:** каталог platform/config metadata для already-resolved types.

Эти компоненты НЕ являются отдельным semantic pipeline. Их роль:

- обогатить canonical owner/type документацией, методами и свойствами;
- сопоставить facet-aware type с platform/config catalog;
- выдать definition location там, где она уже выражена canonical fact'ом.

Файлы:

- [shared/src/domain/metadata_lookup/search.rs](../../shared/src/domain/metadata_lookup/search.rs)
- [bsl-repository/src/signature_index/method.rs](../../bsl-repository/src/signature_index/method.rs)
- [bsl-repository/src/signature_index/types.rs](../../bsl-repository/src/signature_index/types.rs)

### Config Metadata Parser

**Назначение:** загрузка configuration raw types и canonical metadata anchors.

Недавнее изменение в этом слое:

- XML path метаданных теперь сохраняется в `RawTypeData.metadata_path`;
- definition path для configuration symbols может быть построен из canonical facts без consumer-local scan.

Файлы:

- [bsl-runtime/src/data/loaders/config_metadata_parser/parser.rs](../../bsl-runtime/src/data/loaders/config_metadata_parser/parser.rs)
- [bsl-runtime/src/data/loaders/config_metadata_parser/types.rs](../../bsl-runtime/src/data/loaders/config_metadata_parser/types.rs)
- [bsl-runtime/src/data/loaders/config_metadata_parser/converter.rs](../../bsl-runtime/src/data/loaders/config_metadata_parser/converter.rs)
- [bsl-types/src/types/raw_data.rs](../../bsl-types/src/types/raw_data.rs)

## 7. Adapter Layer

Adapters теперь transport-only wrapper над shared runtime.

### LSP

- [backend/src/bin/lsp_server/handlers/completion.rs](../../backend/src/bin/lsp_server/handlers/completion.rs)
- [backend/src/bin/lsp_server/handlers/signature_help.rs](../../backend/src/bin/lsp_server/handlers/signature_help.rs)
- [backend/src/bin/lsp_server/handlers/definition.rs](../../backend/src/bin/lsp_server/handlers/definition.rs)
- [backend/src/bin/lsp_server/server/language_server/helpers.rs](../../backend/src/bin/lsp_server/server/language_server/helpers.rs)

### Web

- [backend/src/presentation/web/handlers.rs](../../backend/src/presentation/web/handlers.rs)

### MCP

- [bsl-agent/src/session/manager_semantic_core.rs](../../bsl-agent/src/session/manager_semantic_core.rs)
- [bsl-agent/src/session/manager_semantic_navigation.rs](../../bsl-agent/src/session/manager_semantic_navigation.rs)
- [bsl-agent/src/session/helpers_semantic.rs](../../bsl-agent/src/session/helpers_semantic.rs)

### CLI

- [cli/src/main.rs](../../cli/src/main.rs)
- [cli/src/runtime.rs](../../cli/src/runtime.rs)

## 8. Observability and Contracts

Shared observability contract фиксирует:

- bounded fail-closed reason codes;
- shared stage taxonomy;
- anti-rescue counters для perf gates;
- separate legacy/internal `type_index_reason_*` drilldown metrics.

Основные артефакты:

- [contracts/mcp-bsl-agent-semantic/v1/contract.json](../../contracts/mcp-bsl-agent-semantic/v1/contract.json)
- [contracts/observability-completion-v2/v3/contract.json](../../contracts/observability-completion-v2/v3/contract.json)
- [contracts/intellisense-perf-gate/v2/contract.json](../../contracts/intellisense-perf-gate/v2/contract.json)
- [docs/guides/lsp-v2-latency-policy.md](../guides/lsp-v2-latency-policy.md)

## 9. Representative Evidence

Актуальная human-readable матрица и machine-readable validation assets:

- [openspec/changes/refactor-ir-canonical-semantic-pipeline/execution-matrix.md](../../openspec/changes/refactor-ir-canonical-semantic-pipeline/execution-matrix.md)
- [openspec/changes/refactor-ir-canonical-semantic-pipeline/validation/acceptance-report.json](../../openspec/changes/refactor-ir-canonical-semantic-pipeline/validation/acceptance-report.json)
- [openspec/changes/refactor-ir-canonical-semantic-pipeline/validation/quality-gates.json](../../openspec/changes/refactor-ir-canonical-semantic-pipeline/validation/quality-gates.json)
