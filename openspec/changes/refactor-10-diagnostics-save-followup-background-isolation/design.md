## Context
После `refactor-09` save pipeline уже truthfully показывает post-fastlane tail как request-centric follow-up breakdown, но свежий bundle `2026-04-08T09-34-09Z` подтвердил, что remediation ещё не завершена:

- `save_fastlane` first publish bounded (`47ms`);
- `idle_heavy` follow-up publishes only after `75848ms`;
- trace already attributes the tail as `runtime_queue_wait_ms=69712`, `apply_lag_ms=3006`, `semantic_diagnostics_query_ms=6126`.

Read-only разбор по коду дал ту же картину:

- `didSave` re-arms same-version shadow/ready artifacts and schedules both save profiles;
- writer-owned applied-state branch still reaches `snapshot_with_deps()` through generic `RuntimeQueuePriority::Background`, so one valid didSave follow-up path would escape isolation unless the contract is explicit;
- post-fastlane follow-up shadow-path, ready-artifacts path and didSave-only generic fallback all still use generic `CpuWorkClass::Background`;
- didSave-only generic fallback additionally uses generic `RuntimeQueuePriority::Background` for `wait_for_file_version` / `snapshot_with_deps`, so the fallback path still inherits generic writer/runtime queue backlog;
- facade/runtime scheduler is currently binary end to end: two `std::sync::mpsc` queues (`Interactive` / `Background`) plus binary `RuntimeQueuePriority` routing;
- `CpuBoundBudget` is global and only distinguishes `Interactive` vs `Background`, with a shared pool and no fairness by profile/origin;
- runtime saturation/latency projections are also binary today, so dedicated follow-up attribution would be lost if the new lane were encoded only as another `work_class` string;
- writer-owned applied-state follow-up currently bypasses both generic `prepare_stateful_operation(...)` and observed bounded blocking admission, so it would remain an architectural escape hatch unless it is brought under the same lane contract;
- unrelated auxiliary/background work such as `bsl.getCurrentContext`, parse-snapshot enrichment, type-index precompute and other background diagnostics stages can therefore legally strand didSave follow-up behind the same background lane.

For this incident, writer/apply is no longer the primary blocker. It remains visible and worth a later hardening pass, but the authoritative save trace shows `runtime_queue_wait` dominating `apply_lag`.

## Goals
- Isolate post-fastlane `didSave + idle_heavy` follow-up from generic background runtime backlog by default.
- Preserve bounded `save_fastlane` first publish semantics unchanged.
- Keep heavy follow-up non-interactive, supersession-safe, and truthful in request-centric observability.
- Make the regression reproducible and acceptance-testable under representative background co-saturation.

## Non-Goals
- Do not redesign the full writer queue or the entire background fairness architecture.
- Do not promote heavy follow-up to the same class as completion/hover/definition.
- Do not reopen extension/UI-first investigation without fresh contradictory evidence.
- Do not solve process-wide `apply_change_set_file_exec_ms` outliers in this change.

## Decisions

### 1. Post-fastlane didSave follow-up gets an explicit end-to-end bounded lane
The primary fix is a first-class dedicated bounded admission lane for post-fastlane didSave heavy follow-up, orthogonal to the existing binary `CpuWorkClass` taxonomy, not a local `if didSave` special case inside diagnostics runtime.

This lane must span both:

- blocking CPU admission for shadow/ready/syntax/semantic work;
- didSave fallback scheduling through writer/runtime queue preparation.

The lane identity must be carried explicitly through the facade/runtime contract. It MUST NOT be inferred only from `SemanticOperation::Diagnostics`, because that would keep didSave follow-up coupled to every other diagnostics-shaped background operation.

`SemanticOperation::Diagnostics` remains the operation kind, but didSave heavy follow-up lane identity is a separate admission concern that must be propagated explicitly through facade/runtime APIs. Writer/runtime preparation and blocking CPU execution must consume the same lane identity end to end.

Recommended concrete shape: a first-class lane marker such as `AdmissionLane::DidSaveFollowup` (or a semantically equivalent type) with canonical additive telemetry/raw-label value `did_save_followup`. The exact type name is flexible, but the contract must distinguish lane identity from both `SemanticOperation` and `CpuWorkClass`.

It must:

- remain distinct from `Interactive`;
- keep `CpuWorkClass` semantics binary (`Interactive` / `Background`) and treat didSave follow-up lane identity as a separate admission concern layered on top of existing non-interactive/background CPU accounting instead of minting a third work class;
- avoid borrowing interactive reserved capacity;
- avoid inheriting generic background backlog as the default admission gate;
- be carved out of the existing bounded runtime/CPU budget rather than minting net-new total process-wide parallelism;
- expose one operator-visible quota as the number of end-to-end didSave follow-up slots rather than separate configurable writer and CPU quotas for the same work;
- remain bounded so that save storms do not create unbounded parallelism.

This is intentionally stronger than a simple priority tweak inside the existing `Background` class. Current permits are held for the full `spawn_blocking(...).await` duration, so queue reordering alone cannot protect follow-up from long in-flight background holders. The same principle applies to fallback prepare stages that currently inherit generic background writer/runtime queue policy.

This also rules out a "nominal lane only" implementation where the server just attaches a new enum/tag to work that still enters the same generic `Background` FIFO and the same generic `Background` permit wait queue. A lane identity without a pre-admission gate does not satisfy the contract because supersession and `quota=0` would still be observed too late, after scarce capacity was already consumed.

The operator-visible quota is intentionally defined as a single end-to-end slot contract. One admitted heavy follow-up consumes exactly one didSave-follow-up slot from outer admission through writer/runtime preparation, blocking CPU execution and the final pre-publish supersession/quota/disposition decision. The implementation MUST release that scarce slot before outbound publish/output wait so publish-path contention remains separately observable and does not monopolize the dedicated lane. The implementation MUST NOT expose or rely on independent writer-vs-CPU lane quotas for the same follow-up because that would make capacity accounting ambiguous and could accidentally increase effective parallelism.

### 2. All post-fastlane heavy branches, including applied-state, are part of the primary remediation
The isolation contract must cover the whole post-fastlane didSave follow-up, not only one happy-path branch.

That includes:

- writer-owned applied-state follow-up when exact same-version applied state already exists but richer diagnostics still need `snapshot_with_deps` plus semantic work;
- same-version shadow-state follow-up;
- same-version ready-artifacts follow-up;
- didSave-only fallback preparation through `wait_for_file_version` / `snapshot_with_deps`;
- didSave-only generic syntax/semantic fallback when the fast paths are unavailable.

Otherwise the regression would survive through whichever branch still escapes the lane and remain workload-dependent.

Architecturally this means the implementation needs one didSave-follow-up lane label shared by applied-state, shadow-state, ready-artifacts and fallback handling while keeping CPU-class accounting aligned with existing non-interactive/background semantics.

The applied-state branch needs especially explicit treatment: today it can jump straight into `snapshot_with_deps()` plus inline syntax/semantic work and therefore escape the same prepare/admission hooks that the generic fallback path uses. The new lane contract must close that bypass instead of treating applied-state as a special direct-call exception.

### 3. The lane is implemented as centralized lane-aware arbitration with one owner above branch fan-out
The implementation should keep a single writer/runtime scheduler, but the outer didSave-follow-up arbiter itself must have one explicit owner: diagnostics runtime orchestration before branch fan-out into applied-state, shadow-state, ready-artifacts or fallback handling.

This owner choice is intentional:

- branch selection, save-cycle identity, latest received version and supersession facts already live in diagnostics runtime;
- applied/shadow/ready/fallback paths are currently selected there, so putting the arbiter below that point would either duplicate logic or leave branch-local escape hatches;
- facade/runtime APIs still need a first-class lane contract, but they should consume opaque admission facts/token issued by the shared outer arbiter instead of each branch implementing its own queue semantics.

This means:

- fairness and ownership stay centralized instead of duplicating scheduler logic across backend branch handlers or in a separate writer thread;
- per-lane queueing is acceptable as an implementation detail, but the contract is one scheduler with lane-aware arbitration;
- applied-state, shadow-state, ready-artifacts and fallback branches all use the same lane semantics rather than bespoke routing.
- diagnostics runtime owns latest-wins queueing, supersession checks, per-file coalescing and slot issuance before branch fan-out;
- downstream facade/runtime prepare helpers consume lane identity plus opaque admission state and MUST NOT re-invent a second branch-local arbiter;
- lane identity and supersession facts must be part of the command/admission envelope before scarce writer/runtime resources are consumed; generic routing from `SemanticOperation::Diagnostics` to `RuntimeQueuePriority::Background` is not sufficient;
- the implementation therefore needs an explicit outer admission boundary or equivalent lane-owned queue/gate ahead of the existing scarce FIFO/permit points; simply feeding tagged work into the current `Background` channels is insufficient;
- the didSave-follow-up lane re-partitions existing bounded capacity and MUST NOT increase the total number of runtime/CPU permits available process-wide.
- runtime-config quota changes control future outer-lane admission decisions; work that has already crossed admission with a slot may finish under that slot and is not retroactively revoked or reclassified mid-flight.

This choice keeps the remediation local to admission policy and avoids a long-tail of branch-specific queue behavior that would be harder to validate.

### 3a. The outer admission boundary is a latest-wins arbiter with one end-to-end slot guard
The preferred implementation shape is an explicit outer admission arbiter for `didSave` heavy follow-up, ahead of existing writer FIFO and blocking CPU permit waits.

This arbiter should own:

- latest-wins queueing/coalescing by same-file save cycle;
- latest-only queued entry retention per file instead of raw FIFO accumulation of stale same-file work;
- fair rotation across distinct files that currently have queued follow-up work;
- supersession re-check before scarce resource acquisition;
- `quota=0` re-check before scarce resource acquisition;
- one RAII-like end-to-end slot guard per admitted heavy follow-up lifecycle.

That slot guard should be acquired when work crosses the outer admission boundary and should be released after the final pre-publish supersession/quota/disposition decision for that heavy follow-up, before outbound publish/output wait. It therefore spans:

- any writer/runtime preparation needed to obtain same-version state;
- any fallback `wait_for_file_version` / `snapshot_with_deps` work that remains inside the lane contract;
- blocking CPU execution for syntax/semantic follow-up work;
- final pre-publish supersession/disabled disposition decision.

Outbound publish/output wait remains observable through existing request-centric save timeline facts such as `publish_wait_ms`, but it is not part of the scarce `did_save_followup` lane contract and MUST NOT keep the dedicated slot occupied after the heavy follow-up has been materialized and accepted for publish.

This is stronger than "CPU permit only" gating. A raw semaphore around `spawn_blocking` is not sufficient because queued work would still sit in generic runtime/writer scarcity without latest-wins or `quota=0` re-check semantics at the correct boundary.

### 3b. Quota is global process-wide, while queue fairness is latest-only per file plus fair rotation
The didSave-follow-up lane quota is a global process-wide slot count.

That means:

- with effective quota `1`, at most one didSave heavy follow-up lifecycle across all files may hold the lane at once;
- the implementation MUST NOT reinterpret the knob as a per-file slot count or any other multiplicative capacity model;
- global quota still does not authorize a raw FIFO queue of save cycles across files.

Queue fairness for queued-but-not-admitted work therefore follows two additional rules:

- each file contributes at most one queued candidate, representing the latest queued save cycle for that file;
- when more than one file has queued work, the arbiter rotates fairly across distinct files instead of letting one noisy file monopolize the queue with superseded entries.

This is the narrowest model that closes the cross-file head-of-line risk without widening the change into a full background-scheduler redesign.

### 4. `quota = 0` is an explicit disable switch, not a clamp or silent fallback
The runtime-config quota for the didSave follow-up lane uses explicit zero semantics:

- absent override means default effective quota `1`;
- `0` disables new `didSave + idle_heavy` follow-up admissions;
- `0` MUST NOT clamp to `1`;
- `0` MUST NOT silently reroute heavy follow-up back into the generic background lane;
- queued-but-not-started work MUST re-check the effective quota at scarce-lane admission time and finish disabled instead of running on stale pre-disable assumptions;
- positive quota changes also apply at the same outer admission boundary and govern future admissions only;
- `0` MUST produce explicit request-centric terminal facts for the skipped heavy follow-up instead of leaving the cycle in silent absence;
- effective zero is a runtime-mutable admission-gate state, not a request to reinterpret the lane as a third `CpuWorkClass`;
- `save_fastlane` first publish and interactive lanes remain unaffected.

This fail-closed behavior is preferable to silently reintroducing the old contention mode under operator override.

### 5. `save_fastlane` semantics stay unchanged
`save_fastlane` remains the bounded same-version first refresh and is not part of this remediation.

This change must not:

- make first publish slower;
- couple first publish back to writer/apply lag;
- weaken supersession guarantees between consecutive save cycles.

### 6. The new lane is runtime-configurable and observable from day one
This change introduces a stable runtime-config knob for the dedicated didSave follow-up lane quota/permits.

The implementation must also export dedicated telemetry for this lane immediately:

- queue-wait / exec metrics by the new lane label;
- saturation gauges for waiters/permits belonging to the lane;
- request-centric traces that continue to expose `followup_runtime_queue_wait_ms` and `apply_lag_ms`.

Without these metrics the rollout would be blind: the system could move contention from generic background to the new lane without making that visible to operators.

The telemetry contract is fixed more explicitly than before:

- canonical additive lane visibility for this change uses a bounded first-class `lane` surface, or a semantically equivalent dedicated runtime-lane metric family, for queue-wait samples, exec samples and saturation gauges;
- the bounded lane set for this change MUST include stable canonical value `did_save_followup`;
- legacy `interactive/background` / `work_class` metrics remain compatibility projections and MUST NOT be the only emitted representation of lane identity;
- request-centric save timeline MAY omit a literal lane-name field only if no lane-specific fact is lost and operators can still directly distinguish the dedicated follow-up path from generic background contention;
- `quota=0` state must stay visible both in runtime-config snapshot and in request-centric save timeline outcome;
- shared diagnostics terminal taxonomy is explicitly extended with first-class non-cancellation outcome/disposition `disabled_by_config`;
- `disabled_by_config` MUST NOT be normalized into cancellation-only buckets, histograms or compatibility projections.
- downstream human-readable diagnostics save summary in incident bundles MUST preserve `disabled_by_config` as the same explicit non-cancellation terminal outcome instead of degrading it to `pending`, `unknown` or a generic cancellation surrogate.

This observability contract is intentionally introduced in the current change instead of being deferred to `rewrite-v2-observability-perf-pipeline`. The later rewrite may reorganize ingestion/projection internals, but it should inherit this lane/outcome taxonomy rather than block the runtime remediation on a larger observability migration.

### 7. Representative saturation validation must model real co-saturators
The regression is not only "generic background load in the abstract". The same lane is plausibly saturated by concrete auxiliary/background work already present in the product:

- `bsl.getCurrentContext` parse/context derivation;
- parse-snapshot enrichment and same-version refresh;
- type-index precompute;
- other background diagnostics stages.

Validation therefore must include at least one deterministic saturation guard where unrelated background work is active and didSave follow-up still starts in bounded time relative to the old failure mode.

### 8. Save-storm latest-wins behavior is enforced via explicit stale-shedding checkpoints
Once a dedicated lane exists, older heavy follow-up work can become the new head-of-line blocker unless the system sheds stale work before it monopolizes the lane.

Current code already has coalescing/promotion for interactive current-revision apply commands, but that logic does not cover didSave follow-up branches, and started `spawn_blocking` work runs until completion. Therefore the new lane needs explicit stale-shedding checkpoints of its own instead of relying on queue fairness alone.

Architecturally this means:

- the critical admission boundary is before scarce writer/runtime queue capacity is consumed, not only before publish;
- older didSave follow-up work must be rejected before consuming scarce lane capacity when a newer same-file save cycle has already superseded it;
- queued-but-not-started follow-up work must re-check both supersession and effective quota at the admission boundary itself;
- queued work for one file must not create an unbounded FIFO wall for another file; only latest queued work per file survives and fair rotation happens across distinct files;
- the facade/runtime envelope therefore needs explicit lane-plus-supersession facts, or an equivalent outer follow-up admission queue, before generic `wait_for_file_version` / `snapshot_with_deps` work enters scarce FIFO state;
- once work has crossed an admission boundary, the implementation still needs a fresh supersession checkpoint before publish so obsolete follow-up does not hold the lane longer than necessary;
- validation must prove not just eventual supersession, but that older queued follow-up is shed before it becomes the default blocker for newer save cycles.

The observable contract remains latest-wins:

- stale follow-up work does not monopolize the new lane longer than necessary;
- newer save cycles still reach bounded first publish;
- newer heavy follow-up is not stranded behind obsolete older follow-up by default.

### 9. `quota=0` finishes the heavy branch with an explicit non-cancellation outcome
The dedicated lane can be disabled operationally, but that must remain truthful in operator-facing traces.

When `quota=0` and `save_fastlane` first publish has already completed:

- the server MUST finish the heavy follow-up branch with explicit outcome `disabled_by_config`;
- the save trace MUST expose that outcome instead of leaving `idle_heavy` absent or mapping it to a generic cancellation bucket;
- the same outcome MUST be represented canonically in diagnostics pipeline counters / terminal disposition reporting via the shared outcome/disposition contract rather than only as an ad-hoc trace string or `other_cancel`-style normalization;
- this outcome is not a silent fallback and not a substitute for generic background execution.

This keeps runtime-config override behavior diagnosable and prevents operators from confusing intentional suppression with hidden starvation.

### 10. Writer/apply hardening is explicitly deferred
Bundle-level cumulative metrics still show large `apply_change_set_file_exec_ms` outliers, but they are not the primary blocker for this incident's didSave follow-up trace.

This change may keep existing `apply_lag` observability and should preserve truthful attribution, but it does not need to solve coarse writer-side apply occupancy. If that remains a problem after follow-up isolation, it should be handled as a separate change with finer writer sub-phase instrumentation and targeted cache/lock analysis.

## Resolved Assumptions and Open Questions

### Default quota answer
The default effective quota is explicitly set to `1`.

Reasoning:

- it keeps the remediation enabled by default;
- it preserves a bounded single-slot mental model for operators;
- it avoids accidental net-new save-storm fan-out before the lane proves stable under production-like load.

### Outer arbiter ownership answer
The outer admission arbiter is owned by diagnostics runtime orchestration, not by individual follow-up branches and not by a second independent arbiter hidden inside facade/runtime helpers.

Reasoning:

- diagnostics runtime already owns save-cycle identity, supersession and branch selection;
- one owner prevents applied/shadow/ready/fallback divergence;
- the facade/runtime contract still remains first-class, but as a consumed downstream admission surface rather than a duplicate arbitration owner.

### Fairness scope answer
The quota is a global process-wide slot count, while queued work uses latest-only per-file retention and fair rotation across distinct files.

Reasoning:

- a global slot count preserves the "no net-new total parallelism" contract;
- latest-only per-file retention prevents save storms from one file from bloating the queue with stale work;
- fair rotation closes the cross-file head-of-line risk that a naive global FIFO would introduce.

### Observability rewrite coordination answer
`rewrite-v2-observability-perf-pipeline` is not a prerequisite for this change.

Reasoning:

- the current incident needs a runtime scheduling remediation now;
- the current repo already enforces explicit registry/projection updates when observability taxonomy changes;
- introducing a minimal canonical additive lane/outcome surface now reduces ambiguity and gives the later rewrite a concrete contract to preserve.

### Telemetry schema answer
The additive schema for this change is fixed as a bounded first-class lane surface for runtime queue/exec/saturation signals plus explicit terminal value `disabled_by_config` in the shared diagnostics outcome/disposition taxonomy.

Reasoning:

- this avoids overloading binary `work_class` as a third semantic axis;
- it gives implementation work a concrete compatibility target instead of an underspecified "some additive metric";
- it keeps the later observability rewrite constrained to preserve or migrate an already explicit lane/outcome contract.

## Alternatives Considered

### Keep one `Background` class and only boost follow-up priority
Rejected. This improves dequeued ordering at best, but it does not protect against already-running long background holders because current permits are held for the full blocking exec span.

### Add only a new CPU class and leave fallback on generic writer/runtime queue
Rejected. This would fix shadow/ready paths but still leave the didSave fallback path exposed to the exact backlog class that the change is trying to remove.

### Add a new lane tag but keep the same generic background FIFO and permit waits
Rejected. This would rename the work without moving the real admission boundary, so queued stale work and `quota=0` re-checks would still happen after scarce background resources were already consumed.

### Split the dedicated lane into separate configurable writer and CPU quotas
Rejected. The operator-facing contract would become ambiguous, validation would have to reason about two partially overlapping bottlenecks, and the implementation could accidentally increase effective parallelism even while each individual quota looked bounded.

### Introduce a third `CpuWorkClass` for didSave follow-up
Rejected. The current CPU-budget partitioning and observability taxonomy are intentionally binary (`Interactive` / `Background`). Overloading that axis would broaden the change into budget/metrics compatibility churn instead of a local admission-isolation remediation.

### Create a standalone third writer scheduler for didSave follow-up
Rejected. The problem is admission/isolation policy, not a need for a second ownership model. Duplicating scheduler logic would make branch behavior drift more likely and would complicate fairness, metrics and rollback.

### Promote heavy follow-up to `Interactive`
Rejected. The goal is not to let save follow-up steal latency budget from true interactive user requests.

### Treat writer/apply as the primary fix target
Rejected for this change. The authoritative per-save trace shows `runtime_queue_wait` as the dominant blocker and `apply_lag` as secondary.

## Risks / Trade-offs

### Risk: the dedicated lane becomes a hidden second interactive pool
Mitigation:
- keep the lane non-interactive;
- avoid borrowing interactive reserved permits;
- cap the lane with a small bounded quota.

### Risk: fallback paths still escape into generic background contention
Mitigation:
- route all post-fastlane didSave blocking branches, including prepare fallback, through the same isolation policy;
- add regressions that exercise both fast paths and forced fallback.

### Risk: save storms starve generic background maintenance
Mitigation:
- keep dedicated follow-up concurrency intentionally small;
- preserve supersession/latest-wins semantics so stale follow-up work is shed before monopolizing the lane.

### Risk: one noisy file monopolizes the new lane or its queue across files
Mitigation:
- define the quota as global process-wide slots;
- retain only the latest queued follow-up per file;
- rotate fairly across distinct files waiting on the lane.

### Risk: split or retroactively mutable quota semantics create hidden parallelism or operator confusion
Mitigation:
- define quota as one end-to-end slot contract for a heavy follow-up lifecycle;
- apply runtime quota changes only at the outer admission boundary for future admissions;
- avoid revoking already admitted work mid-flight.

### Risk: `quota=0` suppresses richer follow-up entirely
Mitigation:
- define zero as an explicit operator override rather than a hidden clamp;
- surface the effective zero value in runtime-config snapshot and lane metrics;
- keep `save_fastlane` unaffected so first publish remains available;
- emit explicit `disabled_by_config` follow-up outcome so suppression is truthful and diagnosable.

### Risk: rollout is opaque because the new lane has no independent metrics
Mitigation:
- add dedicated lane metrics and saturation gauges in the same change;
- expose lane quota through runtime-config snapshot so operator state is observable.

## Validation
1. Regression: unrelated background saturation no longer dominates post-fastlane didSave follow-up as the default primary gate.
2. Regression: writer-owned applied-state follow-up path uses the same isolated lane and no longer inherits generic background runtime queue backlog as the default primary gate.
3. Regression: forced didSave fallback path no longer inherits generic background writer/runtime backlog as the default primary gate.
4. Regression: quota is enforced as a single end-to-end didSave-follow-up slot contract and does not introduce separate independently configurable writer-vs-CPU limits for the same work.
5. Regression: absent override means default effective quota `1`, and default behavior remains bounded without net-new save-storm fan-out.
6. Regression: global quota `1` plus per-file latest-only queueing does not allow one noisy file to build an unbounded FIFO wall for another file, and distinct files receive fair rotation at queued admission time.
7. Regression: `quota=0` disables new `didSave + idle_heavy` admissions explicitly without silently falling back to generic background and without changing `save_fastlane`.
8. Regression: queued-but-not-started follow-up re-checks effective `quota=0` at scarce-lane admission time and finishes `disabled_by_config` instead of running on stale pre-disable assumptions, while already admitted work is not retroactively revoked mid-flight.
9. Regression: `disabled_by_config` is emitted canonically in save trace and in diagnostics pipeline terminal outcome/disposition reporting instead of silent absence or generic cancellation, and it is not classified as a cancellation outcome.
10. Regression: save-storm latest-wins behavior remains intact with the new lane, and stale queued follow-up is shed before becoming the default blocker for newer save cycles.
11. Regression: bounded `save_fastlane` first publish remains intact under the same workloads.
12. Regression: request-centric diagnostics save trace still reports residual `runtime_queue_wait_ms` / `apply_lag_ms` truthfully when they happen, process-level metrics expose the new lane separately via additive telemetry schema with a first-class lane surface, and legacy `CpuWorkClass` / `interactive/background` projections remain binary compatibility views rather than the only representation.
13. Live report: representative `conf_big` save flow shows bounded first publish and materially reduced follow-up queue wait under comparable mixed load.
