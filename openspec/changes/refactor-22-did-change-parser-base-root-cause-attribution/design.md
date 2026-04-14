## Context

After `refactor-21`, same-file `didChange` work is coalesced, but the dominant exact path in the
new bundle still reaches `fallback_reason=stale_parser_base` for ranged edits built from
`shadow_state`.

That label is already better than the old synthetic mismatch reasons, but it is still not enough to
choose the next fix. The current signal does not distinguish whether:

- shadow state outran the newest ready parse snapshot;
- there never was a matching ready snapshot for the shadow revision;
- priming from a matching ready snapshot happened but the parser tree cache still did not match the
  shadow text.

Those are materially different failure classes with different fixes and different risk.

## Goals / Non-Goals

- Goals:
  - explain why cheap parser-base reuse was unavailable for ranged `didChange`;
  - keep the attribution low-cardinality and bundle-friendly;
  - make the next performance change target the dominant miss class instead of guessing.
- Non-Goals:
  - no parser algorithm rewrite;
  - no wait-budget tuning;
  - no new high-cardinality payloads with raw text or free-form strings.

## Decisions

### Decision: keep `stale_parser_base` as the top-level fallback reason

The existing top-level fallback taxonomy remains useful and already powers checked-in evidence.
This change should not replace it.

Instead, `stale_parser_base` should gain a second layer of root-cause attribution that stays
bounded and explains why the runtime could not recover an incremental base.

### Decision: root-cause evidence must describe state, not guesses

The new fields should be derived from real runtime state already available around the miss:

- shadow version for the base text;
- latest ready-snapshot version and whether its text matched shadow text;
- whether priming from ready snapshot was attempted;
- whether tree-cache state still mismatched shadow text after priming.

This avoids speculative strings such as "probably churn" and keeps the evidence actionable.

### Decision: incident bundles must summarize the miss class directly

Operators should not need to open raw JSON and manually reconstruct the state transition. The
bundle summary should expose the dominant miss class and the relevant bounded counters.

## Alternatives Considered

### Only keep `stale_parser_base`

Rejected. It is truthful but still too coarse for the next fix.

### Add verbose per-request debug logs

Rejected. Logs are too noisy and too expensive for the default operational path.

## Risks / Trade-offs

- Too many miss classes would recreate the same ambiguity in a different form.
- The attribution must remain valid under coalesced same-file churn, where ready state and shadow
  state can legitimately diverge for a short window.

## Migration Plan

1. Add low-cardinality miss classes plus bounded base-state fields.
2. Surface them in incident-bundle export and test-only observability payloads.
3. Capture live evidence on the same real module that produced the latest incident.
