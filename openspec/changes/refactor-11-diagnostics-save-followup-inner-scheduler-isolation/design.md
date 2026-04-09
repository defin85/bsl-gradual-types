## Context
`refactor-10` уже зафиксировал outer didSave-follow-up lane, global quota, additive lane telemetry и truthful `disabled_by_config`. Bundle `2026-04-09T06-51-37Z` и code audit показали, что этого недостаточно:

- outer slot действительно существует и держится до final pre-publish decision;
- но `AdmissionLane::DidSaveFollowup` внутри runtime остаётся в основном telemetry surface;
- facade runtime по-прежнему бинарный: `Interactive` / `Background`;
- blocking CPU budget по-прежнему арбитрируется только по `CpuWorkClass`;
- generic background jobs вроде `bsl.getCurrentContext` всё ещё занимают те же background permits, за которыми позже ждёт admitted follow-up.

Итог: outer admission изолирован, inner scheduler нет.

## Goals
- Убрать loophole, при котором admitted didSave follow-up повторно попадает в generic background contention после outer admission.
- Сохранить один owner outer arbiter в diagnostics runtime и существующий request-centric trace contract.
- Не увеличивать total bounded runtime/CPU parallelism и не заимствовать interactive reserved capacity.
- Сохранить current operator model: тот же quota knob, тот же `disabled_by_config`, тот же additive lane label.

## Non-Goals
- Не вводить отдельный writer thread или отдельный executor для didSave follow-up.
- Не менять semantics `save_fastlane`.
- Не переделывать scheduling policy всех generic background jobs.
- Не переоткрывать UI-first investigation.

## Decisions

### 1. Outer slot MUST own inner execution entitlement
didSave follow-up slot считается admitted только тогда, когда outer guard владеет не только diagnostics-runtime slot, но и opaque inner execution entitlement, вырезанным из existing non-interactive budget.

Этот entitlement:

- удерживается от outer admission через writer/runtime preparation, blocking CPU execution и final pre-publish supersession/disposition decision;
- освобождается до outbound publish/output wait, как и текущий outer slot;
- не доступен generic background work;
- не borrow-ит interactive reserved capacity;
- не увеличивает total permits, а лишь пере-partition-ит существующую bounded non-interactive capacity.

Практический смысл: admitted follow-up больше не должен заново вставать в generic `Background` CPU permit queue.

### 2. Writer scheduler stays single-threaded but becomes lane-aware inside background work
Writer/runtime scheduler остаётся одним и тем же thread/loop. Новый change не вводит отдельный writer scheduler.

Но внутри non-interactive path admission envelope для `did_save_followup` MUST влиять на dequeue order:

- `Interactive` requests сохраняют текущий приоритет;
- admitted `did_save_followup` prepare commands идут раньше generic background backlog;
- generic background сохраняет существующую FIFO/fairness policy между собой.

Это закрывает loophole, где follow-up после outer admission всё ещё ждёт позади unrelated background writer commands.

### 3. Generic background work remains generic
`bsl.getCurrentContext`, auxiliary parse/enrichment и прочая generic background work остаются generic background work.

Исправление достигается не promoted routing этих jobs, а тем, что:

- they do not consume reserved didSave-follow-up entitlement;
- admitted follow-up does not compete with them for the same reserved inner capacity;
- residual contention outside reserved slice остаётся observable.

### 4. Binary `CpuWorkClass` stays intact
`CpuWorkClass` remains `Interactive | Background`.

Inner isolation MUST NOT вводить третий class. Вместо этого scheduler использует two-level model:

- compatibility/public work class остаётся бинарным;
- dedicated didSave-follow-up entitlement является orthogonal admission concern внутри non-interactive budget.

### 5. Telemetry remains additive and truthful
`lane=did_save_followup` остаётся canonical additive telemetry surface.

После исправления trace/metrics MUST продолжать показывать:

- outer lane queue wait;
- residual apply lag;
- residual publish wait;
- dedicated lane saturation.

Но implementation MUST NOT считать задачу выполненной, если lane опять служит только label-ом без реального inner arbitration effect.

## Alternatives Considered

### A. Keep outer lane and telemetry only
Rejected. Это текущий defect: bundle уже показывает, что outer admission без inner reservation не убирает `runtime_queue_wait`.

### B. Introduce third `CpuWorkClass`
Rejected. Это ломает уже принятый binary compatibility contract и расширяет surface сильнее, чем нужно.

### C. Separate writer thread / dedicated executor
Rejected. Это создаёт второй scheduler, усложняет ownership и рискует незаметно добавить net-new parallelism.

## Risks / Trade-offs
- Generic background throughput может немного просесть, потому что часть non-interactive budget станет реально зарезервированной под didSave follow-up.
- Writer/apply lag полностью не исчезнет: in-flight writer work по-прежнему может быть отдельным blocker, но это уже отдельная observable причина.
- Положительные quota values выше safe default `1` должны по-прежнему obey existing bounded budget; implementation MUST cap actual concurrent admissions existing capacity constraints instead of minting new parallelism.

## Migration / Rollback
- Roll forward: включить inner entitlement под existing didSave-followup lane без изменения public knobs.
- Rollback: убрать inner entitlement reservation и вернуть old generic-background behavior, сохранив current outer telemetry contract.

## Open Questions
- Нет архитектурных open questions. Основной remaining choice implementation-level: как минимально протащить opaque entitlement через facade/runtime APIs без разрастания public surface.
