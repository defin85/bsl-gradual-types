# Change: split completion prepare into lightweight current-revision path and exact path

## Why
После `refactor-current-revision-readiness-fast-lane` стало видно, что проблема уже не сводится только к `didOpen/didChange` handoff и post-handoff readiness.

Даже если current revision быстро становится `applied` и `CompletionHeadArtifact` публикуется раньше slow enrich path, completion по default runtime path все еще слишком рано входит в generic `prepare_stateful_operation` контракт:
- canonical path для adapters остается `wait_for_version -> snapshot_with_deps -> deps guard`;
- это делает first response слишком зависимым от heavy exact prepare boundary, даже когда для member-access completion уже достаточно current-revision head truth;
- попытка превратить текущий `AnalysisV2` в долгоживущий shared snapshot для обхода этой зависимости архитектурно неверна, потому что `AnalysisV2` не является detached immutable read model.

Следовательно, следующий ближайший шаг должен не публиковать еще один shared snapshot runtime state, а разрезать prepare boundary на:
- lightweight current-revision prepare для first completion response;
- exact stateful prepare для full semantic path.

## What Changes
- Добавить в `bsl-intellisense-v2` явный контракт split-prepare для completion:
  - lightweight current-revision path для first response;
  - exact path для full semantic truth и exact upgrade.
- Зафиксировать, что member-access completion по default path MUST сначала пытаться использовать lightweight current-revision boundary и MUST NOT требовать full `snapshot_with_deps` как обязательный prereq для `head_hit`.
- Сохранить `prepare_stateful_operation` как heavy exact contract для `hover`, `definition`, `signatureHelp`, `type-at-position` и exact completion path.
- Зафиксировать safe boundary contents для lightweight path:
  - допускаются только узкие immutable request-scoped DTO/read-model данные;
  - запрещается публиковать или кэшировать долгоживущий shared `AnalysisV2` как feature boundary.
- Расширить representative gate так, чтобы он доказывал не только `ok_non_empty`, но и то, что first response реально проходит по lightweight head route, а не продолжает зависеть от heavy generic prepare.

## Impact
- Affected specs:
  - `bsl-intellisense-v2`
- Affected code (implementation follow-up):
  - `backend/src/bin/lsp_server/server/core/execution_context.rs`
  - `backend/src/bin/lsp_server/server/language_server/impl_completion.rs`
  - `backend/src/bin/lsp_server/server/core/deps_and_precompute.rs`
  - `bsl-runtime/src/application/intellisense_v2/facade/operations.rs`
  - `bsl-runtime/src/application/intellisense_v2/facade/runtime.rs`
  - `bsl-runtime/src/application/intellisense_v2/policy.rs`
  - representative live perf / acceptance harness

## Non-Goals
- Не вводить detached immutable published head snapshot в этом change.
- Не переписывать runtime на multi-writer или общий lock-free reader graph.
- Не возвращать stale fallback или degraded semantic substitute.
- Не менять exact-or-fail-closed contract для `hover`, `definition`, `signatureHelp`, `type-at-position`.
