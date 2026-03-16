## Context

Текущее состояние после последних perf-fix:
- canonical `ExactSemanticArtifact` для representative real module (`examples/conf_big/.../Module.bsl`) больше не тратит десятки секунд на source recovery;
- но даже после этого exact artifact остаётся порядка `1.1-1.5s`;
- interactive wait budget для completion по умолчанию равен `120ms`;
- первый current-revision member-access completion на real-module gate остаётся `fail_closed`, хотя hot path уже не должен ждать секунды на общий pipeline.

Это означает, что проблема стала архитектурной:
- correctness contract требует current revision и запрещает stale substitute;
- exact artifact остаётся слишком дорогим для first response;
- дальнейшее “ужимание exact в 120ms” — слишком рискованная и малореалистичная стратегия.

## Goals / Non-Goals

- Goals:
  - обеспечить current-revision non-empty first response для member-access completion на representative large module;
  - сохранить strict no-stale contract;
  - сохранить `hover`, `definition`, `signatureHelp`, `type-at-position` в exact-or-fail-closed режиме;
  - сделать root-cause latency различимым через bounded observability.

- Non-Goals:
  - не вводить stale/degraded/discovery-backed semantic substitute;
  - не переводить все interactive semantic операции на новый fast artifact;
  - не менять смысл `candidate_id`/`completionItem/resolve` на legacy fallback;
  - не расширять runtime knob surface без явной необходимости в первой фазе.

## Architecture Drivers

- Latency: первый completion response должен укладываться в UX-budget на больших реальных модулях.
- Correctness: current revision MUST оставаться обязательной, stale semantic payload запрещён.
- Maintainability: архитектура должна явно разделять fast completion path и full exact path.
- Operability: observability должна показывать отдельные head/exact стадии и причины fail-closed.

## Alternatives Considered

### Option A: Поднять interactive wait budget и оставить exact-only completion

Плюсы:
- минимальные structural changes;
- сохраняется один artifact.

Минусы:
- user-visible latency вырастет до сотен миллисекунд или секунд;
- representative real-module workload всё равно останется на грани бюджета;
- contract останется хрупким к следующим perf regressions.

Вердикт: отклонено.

### Option B: Продолжать микрооптимизации exact artifact до `<=120ms`

Плюсы:
- сохраняется самый строгий semantic contract;
- меньше новых сущностей.

Минусы:
- после уже найденного major bottleneck exact path всё ещё на порядок дороже бюджета;
- потребуется всё более инвазивная оптимизация внутренних semantic фаз;
- высокий риск усложнения invalidation/cancellation ради малореалистичной цели.

Вердикт: отклонено как основной путь.

### Option C: Dual-artifact canonical completion path

Идея:
- ввести дешёвый `CompletionHeadArtifact`, строящийся только из canonical IR текущей revision;
- exact artifact оставить отдельным и полным;
- completion может отвечать из `CompletionHeadArtifact`, если `ExactSemanticArtifact` ещё не ready;
- остальные interactive semantic операции остаются exact-only.

Плюсы:
- соответствует no-stale/current-revision contract;
- даёт быстрый first response без legacy fallback;
- оставляет exact artifact и дальше полезным для enrichment и других операций;
- совпадает с индустриальным prior art: fast local/open-file artifact + background/full index.

Минусы:
- добавляет второй derived artifact и усложняет invalidation;
- требует отдельного observability/scheduling контракта;
- потребует явного reconciliation между head response и exact enrichment.

Вердикт: рекомендуется.

## Decisions

### Decision: Completion использует bounded set canonical current-revision artifacts

Для completion вводится ограниченный набор canonical derived artifacts:
- `CompletionHeadArtifact`
- `ExactSemanticArtifact`

Оба артефакта:
- строятся только из canonical IR snapshot той же revision;
- invalidated по `(file_id, file_version, deps_id, settings_id)`;
- не могут использовать stale payload другой revision.

### Decision: CompletionHeadArtifact ограничивается first-response задачами

`CompletionHeadArtifact` в первой фазе отвечает только за initial completion response, в первую очередь для member-access completion. Он НЕ становится общим semantic substitute для `hover`, `definition`, `type-at-position` или `diagnostics`.

### Decision: Exact artifact остаётся обязательным для non-completion interactive semantics

`hover`, `definition`, `signatureHelp`, `type-at-position` сохраняют exact-or-fail-closed contract. Это удерживает scope change узким и не размывает semantic guarantees по всему стеку.

### Decision: Wait semantics для completion становятся `head-or-exact-or-fail-closed`

Completion request:
1. ждёт bounded время current-revision canonical artifact;
2. если ready `CompletionHeadArtifact`, возвращает current-revision response;
3. если ready `ExactSemanticArtifact`, может использовать exact сразу;
4. если не ready ни один — отвечает fail-closed.

### Decision: Exact-precompute получает waiter-aware orchestration

Если есть interactive waiter на ту же `(file_id, file_version, deps_id, settings_id)`, система должна:
- не плодить отдельные конкурирующие exact builds;
- повышать приоритет exact-precompute относительно background-only работы;
- экспортировать bounded observability для `waiter joined`, `exact upgraded`, `deadline hit`, `superseded`.

## Proposed Architecture

### Derived artifacts

1. `CompletionHeadArtifact`
- source: canonical IR snapshot текущей revision;
- scope: fast completion truth для initial candidate enumeration;
- initial rollout: member-access completion;
- содержимое: owner-resolution head data, lexical scope data, current-revision candidate skeletons, достаточные для `candidate_id` и выдачи списка candidates.

2. `ExactSemanticArtifact`
- source: canonical IR snapshot текущей revision;
- scope: full semantic truth;
- используется для exact completion, `resolve`, `hover`, `definition`, `signatureHelp`, `type-at-position`, `diagnostics`.

### Orchestration

На `didOpen` / `didChange`:
1. строится/обновляется canonical IR snapshot;
2. из него в fast path публикуется `CompletionHeadArtifact`;
3. exact precompute идёт отдельно и может завершиться позже.

На completion request:
1. bounded wait на current-revision `CompletionHeadArtifact` или exact artifact;
2. first response приходит из head artifact или exact artifact;
3. если first response уже ушёл из head artifact, exact может:
   - улучшить последующие completion requests той же revision;
   - обслужить `completionItem/resolve` и другие exact consumers;
   - обновить observability как `head_to_exact_upgrade`.

### Observability

Нужно различать как минимум:
- `completion_head_ready`
- `completion_exact_ready`
- `completion_head_hit`
- `completion_exact_hit`
- `completion_head_to_exact_upgrade`
- `completion_fail_closed_no_current_revision_artifact`
- `completion_exact_wait_deadline`

### Acceptance

Representative gate должен мерить отдельно:
- first-response availability;
- head latency budget;
- exact upgrade latency;
- отсутствие stale substitute.

Synthetic harness остаётся полезным, но не считается достаточным доказательством для этой архитектурной цели.

## Risks / Trade-offs

- Dual-artifact invalidation сложнее, чем single-artifact.
- Есть риск незаметно превратить `CompletionHeadArtifact` в новый “почти semantic fallback”; это запрещено и должно быть отсечено spec/test contracts.
- Если `CompletionHeadArtifact` будет слишком богатым, exact path начнёт дублироваться.
- Если exact enrichment не будет стабильно привязан к `candidate_id`, можно получить drift между first response и resolve.

## Rollout Notes

- Фаза 1: member-access completion на `CompletionHeadArtifact`, exact-only для остальных interactive semantic операций.
- Фаза 2: по результатам perf evidence можно отдельно решать, нужно ли расширять head artifact на другие completion формы или менять runtime knobs.

## References

- rust-analyzer architecture: https://rust-analyzer.github.io/book/contributing/architecture.html
- clangd design/indexing: https://clangd.llvm.org/design/ and https://clangd.llvm.org/design/indexing
- Salsa tuning/cancellation: https://salsa-rs.github.io/salsa/tuning.html and https://docs.rs/salsa/latest/salsa/enum.Cancelled.html
