## Context

The current p56 representative report after refactor-54 separates two paths that were previously
easy to conflate:

- diagnostics follow-up can publish from detached diagnostics-ready artifacts quickly;
- canonical live ready snapshot install can still wait tens of seconds for exact type-index
  readiness.

The relevant worker path currently has this order:

```text
record_detached_diagnostics_ready_artifact_v2
wait_for_exact_type_index_before_ready_install_v2
record_ready_parse_snapshot_v2
record_intellisense_v2_ready_parse_snapshot_materialization
```

So a fast detached diagnostics-ready follow-up does not imply that the later
`did_change_ready_snapshot_materialization_ms` histogram is fast. The latter includes canonical
ready-install waiting for exact type-index readiness.

The same code path also captures `source_label` near worker-loop start. A same-version `didSave`
can later promote or mutate an existing `didChange` target. Without explicit original/effective
source evidence, a `did_change_*` histogram can be stale attribution rather than a pure didChange
latency signal.

## Goals

- Make canonical ready-install/type-index wait a first-class residual after detached diagnostics
  recovery.
- Keep detached diagnostics-ready and canonical live ready install visibly separate in evidence and
  acceptance gates.
- Bound or truthfully classify exact type-index wait before canonical ready install.
- Fix or expose source-label drift caused by same-version didSave promotion/retarget.
- Preserve exactness gates for canonical live consumers.

## Non-Goals

- Do not weaken `current_type_index_serve_only_ready` or equivalent exact readiness checks.
- Do not make detached diagnostics-ready artifacts visible as canonical exact state.
- Do not absorb the residual by raising timeout constants.
- Do not require global type-index architecture rewrites before the narrow p56 residual is
  measured and bounded.

## Decision

### 1. Split detached diagnostics-ready from canonical ready install

The implementation should keep detached diagnostics-ready publication available before canonical
ready install. That fast path is useful and accepted for diagnostics follow-up. It must not,
however, hide the fact that canonical ready install is still blocked on exact type-index readiness.

Representative evidence should therefore carry both timelines:

- detached diagnostics-ready publication elapsed/outcome;
- canonical ready-install exact type-index wait elapsed/outcome.

### 2. Instrument the exact type-index wait directly

`wait_for_exact_type_index_before_ready_install_v2` is the narrow wait site. It should expose enough
low-cardinality evidence to distinguish:

- ready success;
- retargeted target epoch;
- superseded/cancelled task;
- latest-version mismatch;
- no matching type-index task;
- type-index task active but not ready;
- parse snapshot metadata missing;
- serve-only readiness blocked;
- exact artifact exists but belongs to the wrong version.

The representative report should include at least the active requested version, task phase,
current canonical ready snapshot version, observed latest version, exact ready boolean, and parse
snapshot metadata state after the wait/probe.

The implementation should prefer reusing or extending the existing bounded exact type-index wait
trace shape used by interactive exact consumers rather than inventing a second unbounded polling
contract. The current ready-install helper loops with `tokio::time::sleep(10ms)` and no local
deadline, so refactor-55 must make the canonical install wait finite or explicitly classified by a
deadline/equivalent envelope in the exported trace.

### 3. Bound by existing readiness envelope or classify truthfully

The change is not satisfied by allowing canonical ready install to spend multiple seconds waiting
silently after detached diagnostics-ready publication. Success requires one of these outcomes:

- canonical ready install reaches exact type-index readiness inside an explicit checked-in envelope
  for the representative p56 same-file flow;
- the target is truthfully superseded, cancelled, retargeted, or latest-version mismatched;
- the report proves a contract-approved blocker such as type-index invalidation or serve-only
  blocked state, with enough evidence to act on it.

The p56 report already carries baseline materialization values (`p50=3226ms`, `p95=3329ms`) and
the current residual values (`p50=42597ms`, `p95=43758ms`). Implementation must choose a concrete
validation ceiling derived from the checked-in baseline before claiming the wait is bounded; leaving
the threshold implicit is not sufficient architecture evidence.

### 4. Use effective source attribution for materialization metrics and lifecycle labels

Materialization metrics, phase metrics, and lifecycle source labels should reflect the effective
target at the time the metric is recorded. If a didSave mutates or promotes a running didChange
same-version target, the final materialization label must not remain silently attributed only to the
initial didChange source.

The implementation should keep both labels when they differ:

```text
original_source=did_change
effective_source=did_save
promotion=did_save_same_version
```

This prevents the p56 histogram from being used as evidence for the wrong source class.

Because the current worker clones the target after debounce and can later read only
`save_cycle_sequence` before detached publication, the implementation must refresh or otherwise
derive the effective target immediately before detached diagnostics-ready publication, canonical
ready install, lifecycle completion, materialization metrics, and phase metrics.

### 5. Keep validation failure evidence actionable

The p56 live report should fail high canonical materialization latency only when it also has enough
phase/source evidence to identify the class. A failing report should say whether the latency is:

- true didChange canonical ready-install/type-index wait;
- didSave-promoted worker attributed to original didChange;
- retarget/supersession/cancellation race;
- type-index task stuck or not promoted;
- serve-only blocked readiness;
- missing instrumentation.

## Alternatives Considered

### Treat refactor-54 as incomplete

Rejected. Refactor-54 acceptance is about save-fastlane, detached diagnostics-ready follow-up,
program-lowering reuse, and terminal semantic path. The current report satisfies that contour while
still exposing a separate canonical ready-install histogram.

### Increase the ready-install wait

Rejected. The reported p50/p95 are already tens of seconds. A larger wait hides the blocked exact
type-index path and does not improve canonical readiness.

### Skip exact type-index wait for canonical install

Rejected. That would weaken canonical live exact readiness and can leak non-exact state into
interactive consumers.

### Keep source attribution as worker-start source

Rejected. Worker-start source is still useful as original-source evidence, but final
materialization metrics need the effective target source after promotion/retarget to avoid
misleading histograms.

## Risks

### Risk: some large edits legitimately require long type-index materialization

Mitigation: allow a truthful classified blocker, but require representative p56 evidence to name
that blocker instead of leaving the histogram unexplained.

### Risk: source-attribution fix changes existing metric series names

Mitigation: preserve original-source evidence in report fields and, if needed, add effective-source
series rather than deleting historical labels without migration.

### Risk: more observability increases report cardinality

Mitigation: keep labels low-cardinality: source class, wait outcome, task phase, readiness state,
and blocker class. Avoid file paths, hashes, or unbounded diagnostic text in metric labels.
