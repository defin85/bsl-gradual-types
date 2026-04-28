## Context

`refactor-57-runtime-saturation-contract-completeness` fixed the telemetry
contract hole found in the previous bundle. The follow-up bundle at
`/home/egor/code/temp/bsl-observability-incident-2026-04-26T21-01-14Z` is
therefore useful precisely because the saturation taxonomy is clean:
`observability_contract_violation_total=0`, invalid saturation metrics are
absent, and lane gauges are present.

The residual latency is not on the completion ingress path. The affected bundle
contains completion probes with tiny client pre-send waits, and the slowest
completion is dominated by normal collection work rather than seconds of
transport or UI delay. At the same time, save follow-up and current-context
signals point at backend readiness/install contention:

- same-file `didSave` follow-up trace 2: `followup_total=2440ms`,
  `ready_install=2193ms`, `snapshot_with_deps=1949ms`, `parse_exec=84ms`;
- same-file `didSave` follow-up trace 1: `followup_total=1225ms`,
  `snapshot_with_deps=382ms`, semantic query `840ms`;
- cumulative runtime metrics: `runtime_snapshot_with_deps_queue_wait_ms`
  `p95=3881ms`, `runtime_wait_for_file_version_exec_ms` `p95=2680ms`;
- concurrent `bsl.getCurrentContext` entries appear as completion contenders
  with ages up to `3485ms`, but the bundle lacks a first-class per-request
  current-context section that explains their route and outcome.

## Goals

- Preserve completion-path isolation while investigating readiness contention.
- Make `bsl.getCurrentContext` requests attributable by route, generation,
  broker role, wait result, parse source, supersession/budget outcome, and wall
  time.
- Split didSave follow-up readiness waits into explicit buckets so a seconds
  scale `ready_install` residual cannot hide behind a generic label.
- Bound or short-circuit stale current-context and readiness work when a newer
  generation supersedes the request, without losing correct latest results.
- Keep representative observability bundles contract-clean after the fix.

## Non-Goals

- No production implementation should begin until this OpenSpec change is
  approved.
- This change does not redesign the full observability pipeline.
- This change does not replace the existing current-context broker with a new
  subsystem unless implementation evidence proves the local path cannot be
  bounded.
- This change does not change completion semantics or weaken exact-version
  correctness for diagnostics.

## Approach

### 1. Instrument first

Add or extend a bounded current-context request timeline surface. Each
`bsl.getCurrentContext` request should export enough low-cardinality evidence to
explain:

- document URI or a bounded document identity already used in incident bundles;
- requested generation/version and whether a newer generation superseded it;
- selected route: ready snapshot, latest-only stabilization, parse broker
  leader/follower, fallback, or budget exhaustion;
- wait budgets and elapsed times for ready snapshot, broker, parse, and runtime
  operations;
- outcome: served, superseded, shared follower result, timeout/budget exhausted,
  or failed;
- correlation with concurrent completion/didSave windows where available.

The bundle should surface this as first-class request evidence, not only as
derived contender rows in completion traces.

### 2. Split ready-install attribution

Extend didSave follow-up timelines and/or runtime operation traces so
`ready_install` and `snapshot_with_deps` waits are decomposed into stable
operator-facing buckets, for example:

- exact type-index or file-version wait;
- runtime lane queue wait;
- `snapshot_with_deps` queue/exec wait;
- publish/apply lock or output handoff wait;
- superseded/latest-version mismatch before exact install;
- unclassified residual.

Acceptance should fail if a seconds-scale residual is present only as a generic
`ready_install` number with no lower-level bucket.

### 3. Bound stale work

Audit the existing current-context ready-snapshot, latest-only stabilization,
and parse-broker paths. The intended behavior is:

- equivalent same-generation bursts share one expensive leader;
- requests for stale generations stop or downgrade once newer work makes their
  result obsolete;
- followers are bounded by the same request budget and report their final
  route/outcome;
- didSave follow-up does not wait seconds for an exact readiness target after
  the implementation can prove that the target is already superseded,
  impossible, or attributable to a specific runtime blocker.

### 4. Validate with mixed-load evidence

Validation must include both focused tests and a representative bundle or live
metrics snapshot. The important acceptance shape is:

- completions remain healthy under concurrent current-context and diagnostics
  load;
- observability contract violations remain absent or zero;
- current-context requests are visible as their own request class;
- didSave follow-up seconds-scale waits are either removed or classified into
  explicit blocker buckets.

## Risks

- Adding high-cardinality labels to metrics would make the observability surface
  noisy. Prefer bounded enum-like fields and detailed per-request JSON sections
  in incident bundles.
- Aggressively cancelling current-context work can regress useful hover/context
  results. Latest-only behavior must be tied to explicit generation evidence.
- Budget widening can mask the incident without fixing it. Validation must
  reject unclassified residual waits.
