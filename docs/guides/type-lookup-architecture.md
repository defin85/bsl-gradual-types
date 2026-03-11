# Архитектура Type Lookup в canonical semantic pipeline

## Назначение

Этот документ описывает текущий production-путь поиска типа и owner/member truth в `IntelliSense v2`
после change `refactor-ir-canonical-semantic-pipeline`.

Ключевой инвариант:

- semantic truth рождается только из canonical IR snapshot текущей revision;
- fast lookup допускается только как IR-derived projection того же snapshot;
- runtime и adapters не имеют права достраивать semantic answer из текста, `parse_result`,
  discovery/search индексов или stale артефактов.

## Краткая схема

```text
source text
  -> parse_result (syntax-only extraction, incremental parse state)
  -> SemanticProgram + semantic_facts
  -> exact current-revision semantic artifact
  -> shared runtime queries
  -> LSP / Web / MCP / CLI transport
```

В production используются два связанных слоя одного snapshot:

1. `SemanticProgram` и `semantic_facts`
   - canonical IR, где materialize-ятся типы выражений, receiver/member facts,
     binding facts и definition anchors;
2. exact semantic artifact (`type_at_byte_offset_serve_only`, owner hints, derived lookups)
   - revision-bound projection того же IR snapshot;
   - доступен только для exact current revision;
   - на miss/stale работает fail-closed.

## Что является semantic source of truth

### Canonical слой

Основной semantic source:

- [analysis-v2/src/type_inference_v2.rs](../../analysis-v2/src/type_inference_v2.rs)
- [shared/src/ir/semantic_facts.rs](../../shared/src/ir/semantic_facts.rs)
- [shared/src/ir/program.rs](../../shared/src/ir/program.rs)

Именно здесь materialize-ятся:

- `ExpressionTypeFact`
- receiver/member facts для member access и call surfaces
- binding facts для explicit module-context identifiers
- definition anchors для local/common-module/configuration targets

После недавнего cutover configuration symbols дополнительно несут canonical metadata anchor
через `RawTypeData.metadata_path`, чтобы `definition` не восстанавливал XML path на request-time.

### Derived lookup слой

Тонкий fast-query слой для exact revision:

- [analysis-v2/src/lib/snapshots.rs](../../analysis-v2/src/lib/snapshots.rs)
- [analysis-v2/src/lib/analysis_api.rs](../../analysis-v2/src/lib/analysis_api.rs)

Этот слой:

- строится только из canonical IR snapshot;
- не делает отдельный semantic inference;
- не читает discovery/search read-model как semantic source;
- не обслуживает stale revision;
- возвращает reason-coded fail-closed miss вместо substitute behavior.

Важно: API с именем `type_index`/`serve_only` исторически осталось в коде, но семантически это уже
не "второй источник истины", а exact current-revision projection того же IR snapshot.

## Что НЕ является semantic source of truth

### `parse_result`

`parse_result` остаётся syntax artifact и может использоваться только для:

- incremental parsing;
- syntax diagnostics;
- position extraction;
- recovery slicing для неполного кода.

Он НЕ может:

- определять owner/member truth сам по себе;
- строить completion candidates;
- давать alternate type/definition answer, если canonical semantic artifact недоступен.

### Discovery/Search read-model

`IndexSnapshot` и похожие структуры остаются discovery/search-only слоем:

- полезны для symbol search и text/discovery сценариев;
- не могут backfill-ить `completion`, `hover`, `definition`, `members`, `type-at-position`.

Это разделение явно проверяется acceptance tests:

- [backend/src/bin/lsp_server/server/core/tests.rs](../../backend/src/bin/lsp_server/server/core/tests.rs)
- [bsl-runtime/src/application/type_system/services/completion_service/tests.rs](../../bsl-runtime/src/application/type_system/services/completion_service/tests.rs)
- [bsl-agent/src/session/tests.rs](../../bsl-agent/src/session/tests.rs)

## Как теперь работает lookup

### 1. Type-at-position / owner hints

Базовый type lookup идёт через exact current-revision artifact:

- [analysis-v2/src/lib/analysis_api.rs](../../analysis-v2/src/lib/analysis_api.rs)
- [backend/src/bin/lsp_server/server/language_server/helpers.rs](../../backend/src/bin/lsp_server/server/language_server/helpers.rs)
- [bsl-agent/src/session/helpers_semantic.rs](../../bsl-agent/src/session/helpers_semantic.rs)

Flow-sensitive overlay по-прежнему возможен, но он наслаивается на тот же canonical base contract,
а не заменяет его другой semantic веткой.

### 2. Completion

Completion использует syntax-aware extraction только для позиции/receiver slice, а semantic candidates
читает из shared canonical path:

- [bsl-runtime/src/application/type_system/services/completion_service.rs](../../bsl-runtime/src/application/type_system/services/completion_service.rs)
- [bsl-runtime/src/application/type_system/services/completion_service/context.rs](../../bsl-runtime/src/application/type_system/services/completion_service/context.rs)
- [bsl-runtime/src/application/type_system/services/completion_service/member_resolution.rs](../../bsl-runtime/src/application/type_system/services/completion_service/member_resolution.rs)
- [bsl-runtime/src/application/type_system/services/completion_service/scope_candidates.rs](../../bsl-runtime/src/application/type_system/services/completion_service/scope_candidates.rs)

Для неполного member access canonical owner fact materialize-ится в recovery path ещё на стадии IR build,
чтобы runtime не восстанавливал owner из текста локально.

### 3. Hover / definition / signatureHelp

Interactive semantic consumers на analyzed path читают `semantic_facts` напрямую:

- [bsl-runtime/src/application/type_system/services/hover_service.rs](../../bsl-runtime/src/application/type_system/services/hover_service.rs)
- [bsl-runtime/src/application/type_system/services/definition_service.rs](../../bsl-runtime/src/application/type_system/services/definition_service.rs)
- [bsl-runtime/src/application/type_system/services/signature_help_service.rs](../../bsl-runtime/src/application/type_system/services/signature_help_service.rs)

Это означает:

- `hover` читает type/member truth из canonical facts;
- `signatureHelp` использует serialized callable/receiver facts вместо request-time repository rescue;
- `definition` получает `TypeDefinitionLocation` из canonical facts, включая configuration XML path.

## Роль metadata/repository слоя

Repository, `TypeMetadataLookup` и `SignatureIndex` остаются важными, но их роль теперь ограничена:

- enrich already-resolved semantic owner/type;
- вернуть методы, свойства, сигнатуры и документацию для уже определённого canonical type;
- сопоставить canonical type с platform/config metadata catalog.

Это не второй semantic pipeline.

Ключевые файлы:

- [shared/src/domain/metadata_lookup/search.rs](../../shared/src/domain/metadata_lookup/search.rs)
- [bsl-repository/src/signature_index/types.rs](../../bsl-repository/src/signature_index/types.rs)
- [bsl-runtime/src/data/loaders/config_metadata_parser/converter.rs](../../bsl-runtime/src/data/loaders/config_metadata_parser/converter.rs)
- [bsl-types/src/types/raw_data.rs](../../bsl-types/src/types/raw_data.rs)

## Facet-aware contract

Configuration types обязаны сохранять `active_facet` / `available_facets` на всём пути:

- IR build
- exact semantic artifact
- hover/completion/definition/members runtime
- MCP/Web/LSP transport

Из этого следуют два правила:

- `ЭтотОбъект` / `Объект` остаются canonical explicit bindings там, где модульный контекст это допускает;
- bare identifier fallback для applied owner удалён и не может "магически" резолвиться вне canonical binding model.

Основные места:

- [analysis-v2/src/implicit_bindings.rs](../../analysis-v2/src/implicit_bindings.rs)
- [analysis-v2/src/type_inference_v2.rs](../../analysis-v2/src/type_inference_v2.rs)
- [backend/tests/contextual_implicit_object_matrix_test.rs](../../backend/tests/contextual_implicit_object_matrix_test.rs)
- [backend/tests/form_module_object_unified_contract_test.rs](../../backend/tests/form_module_object_unified_contract_test.rs)

## Fail-closed contract

Если exact current-revision canonical artifacts недоступны, shipped behavior всегда один:

- transport сохраняется;
- semantic payload пустой или unavailable по surface contract;
- observability пишет bounded reason code;
- stale/degraded/search-backed substitute не используется.

Это правило покрывает:

- `completion`
- `hover`
- `signatureHelp`
- `definition`
- `type-at-position`
- `members`

## Проверочные артефакты

Representative evidence лежит в:

- [openspec/changes/refactor-ir-canonical-semantic-pipeline/execution-matrix.md](../../openspec/changes/refactor-ir-canonical-semantic-pipeline/execution-matrix.md)
- [openspec/changes/refactor-ir-canonical-semantic-pipeline/validation/acceptance-report.json](../../openspec/changes/refactor-ir-canonical-semantic-pipeline/validation/acceptance-report.json)
- [openspec/changes/refactor-ir-canonical-semantic-pipeline/validation/quality-gates.json](../../openspec/changes/refactor-ir-canonical-semantic-pipeline/validation/quality-gates.json)

Актуальные acceptance tests для lookup-среза:

- [backend/tests/metadata_completion_fixture_test.rs](../../backend/tests/metadata_completion_fixture_test.rs)
- [backend/tests/m8_completion_matrix_golden_v2_test.rs](../../backend/tests/m8_completion_matrix_golden_v2_test.rs)
- [backend/tests/goto_definition_common_module_test.rs](../../backend/tests/goto_definition_common_module_test.rs)
- [backend/tests/lsp_intellisense_tests.rs](../../backend/tests/lsp_intellisense_tests.rs)
