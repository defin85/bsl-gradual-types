# Change: refactor-v2-completion-dual-artifact-path

## Why
После `refactor-ir-canonical-semantic-pipeline` completion больше не упирается в общий IR/source-recovery bottleneck, но на больших реальных модулях всё ещё системно проигрывает `exact-or-fail-closed` контракту.

На representative real-module gate (`examples/conf_big/.../Module.bsl`) canonical exact precompute после последних оптимизаций занимает примерно `1.1-1.5s`, тогда как interactive wait budget по умолчанию остаётся `120ms`. В результате первый member-access completion для текущей revision остаётся `fail_closed`, хотя canonical current-revision data уже может быть получен значительно дешевле, чем полный exact semantic artifact.

## What Changes
- Ввести второй canonical derived artifact для completion fast path: `CompletionHeadArtifact`.
- Оставить `ExactSemanticArtifact` (current derived semantic index) источником полной semantic truth для `hover`, `definition`, `signatureHelp`, `type-at-position` и exact enrichment completion.
- Изменить contract completion с `exact-or-fail-closed` на `head-or-exact-or-fail-closed`, не нарушая current-revision и no-stale guarantees.
- Добавить scheduler/orchestration правила для согласованной жизни `CompletionHeadArtifact` и `ExactSemanticArtifact`, включая escalation exact-precompute при interactive waiters.
- Расширить observability и perf gates так, чтобы отдельно видеть:
  - first-response availability,
  - head-hit vs exact-hit,
  - head-to-exact upgrade latency,
  - fail-closed по отсутствию current-revision artifacts.

## Impact
- Affected specs: `bsl-intellisense-v2`
- Affected code:
  - `analysis-v2` derived artifacts and invalidation
  - `bsl-runtime` orchestration/runtime policy
  - `backend` LSP completion orchestration and observability
  - perf/live gates for representative real modules
