## Context

After `refactor-14`, the save timeline can show `semantic_path`, `semantic_parse_source`, and
`semantic_ir_source`, but it still cannot answer the key operational question:

"Why did the server not use `ready_artifacts` for this save cycle?"

The current implementation returns an effective `None` from the ready-snapshot probe and then
falls through to `shadow_state`. That is truthful for control flow, but not diagnosable enough for
incident response.

## Goals

- Make `ready_artifacts` misses attributable without reading code.
- Keep attribution low-cardinality and stable across runs.
- Preserve current branch behavior and current fail-closed semantics.

## Non-Goals

- No scheduling or latency optimization in this change.
- No new per-character or raw-error observability payloads.

## Decisions

### 1. Probe outcomes become explicit, not implicit

The timeline should carry canonical outcomes for both:

- zero-budget probe;
- bounded-wait probe.

Canonical miss reasons should be bounded enums such as `not_ready`, `version_mismatch`,
`generation_mismatch`, `timeout`, `cancelled`, and `superseded`.

### 2. Branch-selection context must be visible

The timeline should expose whether `shadow_state` was available and what the same-version
ready-snapshot task state looked like at the moment the runtime chose between `ready_artifacts`
and `shadow_state`.

The task-state contract should stay low-cardinality, for example:

- `absent`
- `in_flight_same_version`
- `in_flight_other_version`
- `ready_same_version`

### 3. Contract evolution must be explicit

The diagnostics save timeline surface already uses versioned payloads. This change must remain
additive and bump the contract so older consumers can mark these fields as
`unavailable_by_design` instead of silently omitting them.

## Risks

- Too many outcome labels would make the contract noisy and hard to compare.
- If the timeline records adapter-local guesses instead of runtime-owned truth, the attribution
  will become misleading.

## Mitigations

- Restrict outcome values to a reviewed bounded enum.
- Source all branch-state fields from runtime-owned structures at the actual decision point.
