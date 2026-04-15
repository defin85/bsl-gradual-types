## Context

Current parse-snapshot observability can tell that `didChange` fell back to full parse, but not
which concrete mechanism failed. That is not enough for an incident-driven workflow, because the
fix is very different depending on whether the problem is:

- no previous tree;
- stale base text for edit derivation;
- malformed or non-applicable edit chain;
- parser rejection after applying edits.

## Goals

- Replace opaque incremental-failure buckets with stable root-cause categories.
- Preserve enough version-bound context to correlate didChange parse failures with later didSave
  snapshot misses.
- Keep payloads low-cardinality and privacy-safe.

## Non-Goals

- No algorithmic change to incremental parsing yet.
- No raw document text, edit payloads, or unbounded error strings in observability.

## Decisions

### 1. Fallback reasons become canonical categories

The parse-snapshot pipeline should use bounded categories such as:

- `no_previous_tree`
- `no_edits_provided`
- `edits_do_not_match_new_content`
- `input_edit_conversion_failed`
- `incremental_parse_failed`
- `other`

### 2. Build context should be recorded at the producer

The most useful context exists where the edit chain is derived and handed to the parser. The
producer should record:

- base-text source (`shadow_state` vs `analysis_snapshot`);
- change shape (`ranged`, `full_replace`, `mixed`);
- canonical fallback reason.

### 3. Operators need version-bound evidence, not only aggregate counters

Aggregate counters remain necessary, but they are insufficient for incident response. The bundle
needs a small version-bound evidence surface so a later didSave miss can be tied back to the
specific didChange parse-snapshot failures that preceded it.

## Risks

- Too many reason categories could make metrics fragmented and hard to compare.
- If producer context is sampled after the fact, it may drift from the actual failed build.

## Mitigations

- Keep the reason taxonomy short and reviewed.
- Record producer context at build time, before any later supersession occurs.
