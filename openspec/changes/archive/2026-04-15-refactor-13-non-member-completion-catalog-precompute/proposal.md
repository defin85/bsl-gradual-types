# Change: precompute immutable non-member completion catalogs

## Why
После того как transport/runtime hotspots будут bounded, свежий completion incident уже локализован значительно точнее: единственный медленный completion (`153ms`) spent почти всё время в `collect`, а новые histograms показывают доминирующие substeps:

- `completion_stage_collect_non_member_global_functions_ms = 80ms`;
- `completion_stage_collect_non_member_repository_types_ms = 60ms`;
- `completion_stage_collect_non_member_metadata_items_ms = 8ms`.

Код в [completion_service.rs](/home/egor/code/bsl-gradual-types/bsl-runtime/src/application/type_system/services/completion_service.rs) подтверждает причину:

- для каждого warm non-member request заново materialize-ятся immutable deps-wide candidate families;
- prefix filtering фактически происходит только позже, на ranking phase, уже после полной materialization candidate vectors;
- local/contextual/module-routine data и правда должны оставаться request-local, но глобальные функции, repository types и metadata items семантически зависят от deps snapshot, а не от cursor-specific state.

## What Changes
- Зафиксировать в `bsl-intellisense-v2`, что warm non-member completion MUST переиспользовать immutable deps-scoped candidate catalogs вместо их полного rebuild на каждый request.
- Потребовать prefix-aware filtering до полной `Candidate` materialization для immutable deps-wide families, если request уже имеет usable lowercase/prefix context.
- Сохранить разделение источников:
  - local/contextual/module-routine candidates остаются revision/context-sensitive;
  - immutable deps-scoped families (`global_functions`, `metadata_items`, `repository_types`, `keywords` или семантически эквивалентные группы) кэшируются и переиспользуются per deps/settings snapshot.
- Добавить representative collect-stage acceptance, чтобы cold/warm non-member collect latency больше не пряталась внутри aggregate completion total.

## Implementation Order
Это третий change в серии. Его стоит делать после `refactor-11` и `refactor-12`, чтобы сначала убрать contention-driven tails, а затем оптимизировать уже локализованный warm collect hotspot.

## Impact
- Affected specs:
  - `bsl-intellisense-v2`
- Affected code (implementation follow-up):
  - `bsl-runtime/src/application/type_system/services/completion_service.rs`
  - `bsl-runtime/src/application/type_system/services/completion_service/context.rs`
  - `bsl-runtime/src/application/type_system/services/completion_service/scope_candidates.rs`
  - `bsl-runtime/src/application/intellisense_v2/facade.rs`
  - `bsl-runtime/src/application/intellisense_v2/facade/runtime.rs`
  - `bsl-runtime/src/system/basic_observability.rs`
  - `backend/src/bin/lsp_server/server/core/tests.rs`
  - representative perf/live evidence under `backend/tests/perf/reports/`

## Non-Goals
- Не менять member-access completion path.
- Не ослаблять lexical-scope correctness для local symbols.
- Не менять ranking semantics или source priorities beyond what is needed for precompute reuse.
- Не лечить runtime/apply contention этим change.

## Resolved Assumptions
- Immutable catalog строится per deps/settings snapshot, а не per file revision, потому что observed hot families зависят от metadata/deps snapshot.
- Prefix-aware filtering должен быть optimisation-only слоем над существующей correctness model; он не должен менять candidate set относительно текущего warm path, кроме latency/memory behavior.
