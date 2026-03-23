# Change: publish current-revision completion head as detached immutable snapshot

## Why
`refactor-completion-prepare-lightweight-exact-split` является рекомендуемым ближайшим шагом, но он сознательно не решает более дорогую долгосрочную задачу: completion first-response path все еще остается привязанным к live runtime boundary и writer-mediated reads.

Даже после split-prepare архитектура будет иметь следующие ограничения:
- source of truth по-прежнему находится за writer thread;
- lightweight path все еще читает current runtime state, а не опубликованный detached read model;
- внешний adapter path остается чувствительным к read-side queue discipline и ownership-модели runtime.

Для долгосрочного снижения coupling нужен отдельный change, который формализует published immutable head snapshot как canonical derived read model, а не как shared `AnalysisV2`.

## What Changes
- Добавить в `bsl-intellisense-v2` контракт detached immutable current-revision head snapshot для completion first-response path.
- Зафиксировать, что этот snapshot:
  - публикуется как отдельный canonical derived read model;
  - keyed по `(file_id, file_version, deps_id, settings_id)`;
  - safe для concurrent readers;
  - не является shared `AnalysisV2` и не держит writer-owned mutable runtime state.
- Разделить producer и consumer responsibilities:
  - runtime/LSP producer публикует detached current-revision head snapshot после canonical current-revision head build;
  - completion consumer читает detached snapshot как first-response payload boundary.
- Сохранить `ExactSemanticArtifact` и heavy exact prepare как отдельный path полной semantic истины.
- Добавить observability и acceptance gate для detached read path, чтобы отличать published head-snapshot availability от writer-queue bound reads.

## Impact
- Affected specs:
  - `bsl-intellisense-v2`
- Affected code (implementation follow-up):
  - `analysis-v2` derived artifacts / storage boundary
  - `bsl-runtime` publication and read-model APIs
  - `backend` LSP completion orchestration
  - representative perf / live readiness gates

## Non-Goals
- Не использовать shared `AnalysisV2` как substitute для detached snapshot.
- Не заменять exact semantic truth detached head snapshot’ом.
- Не делать full Roslyn-style whole-solution immutable graph в рамках этого change.
- Не смешивать этот change с ближайшим split-prepare remediation; detached snapshot оформляется как отдельная следующая архитектурная эволюция.
