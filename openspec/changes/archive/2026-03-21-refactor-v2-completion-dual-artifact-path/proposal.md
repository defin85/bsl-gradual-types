# Change: refactor-v2-completion-dual-artifact-path

## Why
После `refactor-ir-canonical-semantic-pipeline` completion больше не упирается в общий IR/source-recovery bottleneck, но на больших реальных модулях всё ещё системно проигрывает `exact-or-fail-closed` контракту.

Новые live probes уточнили характер проблемы:
- `same-revision warm` path здоров и даёт быстрый `ok_non_empty` completion;
- но `revision-churn` path (`didChange -> completion` на каждой новой revision) продолжает системно уходить в `fail_closed`;
- повторяются два разных miss-mode: bounded prepare timeout и exact wait deadline.

На representative real-module gate (`examples/conf_big/.../Module.bsl`) canonical exact precompute после последних оптимизаций всё ещё заметно дороже interactive wait budget по умолчанию (`120ms`). Поэтому проблема уже не сводится к “первому completion после старта сессии”: completion после каждой новой revision снова становится effectively cold, хотя current-revision fast-path данные могут появляться значительно раньше, чем полный exact semantic artifact.

## What Changes
- Ввести второй canonical derived artifact для completion fast path: `CompletionHeadArtifact`.
- Явно разделить readiness/prepare для completion на fast `head-ready` path и отдельный `exact-ready` path, чтобы current-revision first response не зависел от готовности exact precompute той же revision.
- Оставить `ExactSemanticArtifact` (current derived semantic index) источником полной semantic truth для `hover`, `definition`, `signatureHelp`, `type-at-position` и exact enrichment completion.
- Изменить contract completion с `exact-or-fail-closed` на `head-or-exact-or-fail-closed`, не нарушая current-revision и no-stale guarantees.
- Добавить scheduler/orchestration правила для согласованной жизни `CompletionHeadArtifact` и `ExactSemanticArtifact`, включая debounced/background exact-precompute и escalation exact-precompute при interactive waiters.
- Расширить observability и perf gates так, чтобы отдельно видеть:
  - first-response availability,
  - repeated availability under `revision-churn`,
  - head-hit vs exact-hit,
  - head-to-exact upgrade latency,
  - fail-closed по `prepare timeout` vs `exact deadline`.

## Impact
- Affected specs: `bsl-intellisense-v2`
- Affected code:
  - `analysis-v2` derived artifacts and invalidation
  - `bsl-runtime` orchestration/runtime policy
  - `backend` LSP completion orchestration and observability
  - perf/live gates for representative real modules (`same-revision warm` и `revision-churn`)
