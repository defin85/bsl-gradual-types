## Context

`refactor-58-current-context-ready-install-contention` turned the previous
opaque residual into first-class evidence. The new bundle at
`/home/egor/code/temp/bsl-observability-incident-2026-04-27T08-39-19Z` runs
from git `033ac549`, so it is a valid post-refactor-58 observation rather than
an installed-runtime mismatch.

The new evidence is mixed but useful:

- speed is materially better than the `refactor-08` evidence, where follow-up
  waited `39097ms` for file version and did not reach full publish;
- completion ingress/egress remains bounded;
- current-context requests are finally visible and classified;
- the former v15 `ready_install` residual is no longer the blocker;
- one save cycle still has a multi-second heavy follow-up tail, now localized to
  exact `program_lowering`.

The v15 residual shape is:

```text
save_fastlane first publish: 51ms
idle_heavy full follow-up: 4346ms
followup_readiness_blocker_bucket: snapshot_with_deps
snapshot_with_deps_ms: 47ms
semantic_diagnostics_query_ms: 796ms
parse_exec_ms: 3598ms
exact_ready_snapshot_assembly_ms: 3596ms
program_conversion_ms: 3596ms
program_lowering_ms: 3596ms
timeout_phase: parse_exec
timeout_leaf: program_lowering
relief_valve_outcome: engaged_helped
ready_install_ms: 1ms
```

This is not the same failure class as the older `shadow_state` terminal
fallback. The terminal path is `detached_ready_artifacts`, which is the correct
diagnostics-only endpoint. The failure is that the endpoint arrives after a
seconds-scale exact assembly tail, and the operator-facing bucket still says
`snapshot_with_deps` even though the measured `snapshot_with_deps_ms` is small.

## Goals

- Make post-refactor-58 `program_lowering` tail latency a first-class
  acceptance target.
- Preserve the gains already visible in the new bundle: fast save-first-publish,
  completion isolation, clean saturation integrity, and current-context
  attribution.
- Ensure diagnostics-save evidence cannot accept a seconds-scale exact assembly
  tail behind a generic `snapshot_with_deps` bucket.
- Preserve or restore lowering reuse-plan fields when program lowering
  dominates exact assembly, so implementation can distinguish required full
  rebuild from reuse miss or missing instrumentation.
- Add representative validation that compares against the new clean baseline
  instead of older pre-refactor-58 incident shapes.

## Non-Goals

- Do not optimize semantic diagnostics first. In the new v15 trace semantic
  diagnostics is `796ms`; the larger residual is exact assembly/program
  lowering.
- Do not make detached diagnostics-ready artifacts canonical exact readiness for
  interactive consumers.
- Do not change current-context routing unless new evidence shows it blocks the
  didSave tail directly.
- Do not rely on aggregate metrics alone; the acceptance signal must be
  request-centric and save-cycle-local.

## Decision

### 1. Treat `snapshot_with_deps` as too coarse for this tail

After refactor-58, `followup_readiness_blocker_bucket=snapshot_with_deps` is a
useful top-level bucket, but it is not sufficient when the same trace says:

- `snapshot_with_deps_ms` is small;
- `ready_install_ms` is small;
- `parse_exec` and `program_lowering` are seconds-scale;
- `timeout_leaf=program_lowering`.

The bucket should either be refined for this contour or paired with a
fail-visible exact-materialization sub-bucket such as `program_lowering_tail`.

### 2. Program-lowering reuse evidence is part of acceptance

Older program-lowering changes required reuse outcome, rebuilt/reused units, and
reuse-plan hit flags. The new bundle's v15 incident projection exposes
program-lowering duration but does not expose those reuse fields in the
request-centric section. If the runtime has them, the incident projection should
preserve them. If the runtime does not have them on this path, representative
validation should record that as a gap, not accept a generic bucket.

For this change, "preserve" means the evidence survives the whole operator path:
backend diagnostics-save timeline, VS Code custom request typing, incident-bundle
raw JSON, and the human-readable bundle summary. Backend-only fields are not
enough if the exported bundle used for incident triage drops them.

A required-full-rebuild explanation is only acceptable when the exported trace
proves why reuse was unavailable or unsafe for the exact save family. The minimum
proof is the reuse outcome, rebuilt/reused lowering unit counts, reuse-plan source
and hit flags, and a low-cardinality reason for invalidation, supersession,
cancellation, failure, or continuity loss. Missing reuse fields are not proof of a
required rebuild.

### 3. Bound or truthfully classify

Implementation should satisfy the change by one of these truthful outcomes:

- exact assembly avoids the multi-second program-lowering tail for the
  representative save family;
- the tail remains but is proven necessary for a required full rebuild and is
  classified separately from `snapshot_with_deps` with the reuse evidence listed
  above;
- the save family is superseded, cancelled, failed, or loses continuity before
  exact assembly can publish, with explicit evidence.

Budget widening is not a valid outcome.

### 4. Keep completion and current-context out of the critical path

The new bundle shows current-context leader parses of `3550-5063ms`, but
completion still has max `service_future_to_first_poll_wait_ms=0ms` and output
handoff max `5ms`. Current-context remains useful supporting evidence, not the
primary target for this change.

## Risks

### Risk: some edits legitimately require full lowering rebuild

Mitigation: allow a truthful required-full-rebuild reason, but require it to be
request-centric and save-cycle-local. Do not accept missing reuse evidence as a
successful bound.

### Risk: validation becomes too tied to one sample

Mitigation: gate on the residual shape, not exact line numbers or one absolute
elapsed value: fast first publish, clean integrity, small ready-install, small
snapshot-with-deps measured time, and dominant program-lowering tail.

### Risk: existing program-lowering tests already cover an older contour

Mitigation: keep the new tests explicitly post-refactor-58: terminal path is
`detached_ready_artifacts`, ready-install is not dominant, and the regression is
the exact assembly tail plus missing or failing lowering reuse evidence.
