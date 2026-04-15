## Context

`didChange` currently derives two closely related outputs from incoming LSP content changes:

- the new document text used as `new_content`;
- the parser edit chain used for incremental parsing.

For ranged multi-change requests, both outputs must stay anchored to the same pre-change base
revision. If they diverge in ordering or offset interpretation, incremental parsing sees an edit
chain that cannot reconstruct the same `new_content`, and the system falls back to full parse with
`edits_do_not_match_new_content`.

## Goals

- Eliminate producer-side false positives for `edits_do_not_match_new_content` on valid ranged
  `didChange` input.
- Keep the existing bounded fail-safe full fallback contract intact for genuinely invalid edit
  chains.
- Preserve the low-cardinality observability contract already established by `refactor-16`.

## Non-Goals

- No new parser algorithm or tree-sitter contract changes.
- No new public observability dimensions beyond what `refactor-16` already introduced.
- No `bsl.getCurrentContext` request-shaping changes in this change.

## Decisions

### 1. The producer owns one canonical replay plan

`lsp_did_change` should normalize incoming ranged edits into one canonical ordered replay plan
before it mutates any text and before it derives any parser edit objects.

That plan becomes the single source of truth for both `updated_text` and `parser_edits`.

### 2. Multi-range replay uses reverse document order

When a `didChange` request contains multiple ranged edits, the canonical plan should be applied in
descending document order. That keeps every range anchored to the same pre-change base revision and
avoids shifting later offsets while replay is still in progress.

Full-document replacement changes remain outside this normalization rule because they already define
the full target text directly.

### 3. False replay-order mismatches are a producer bug, not a parser failure class

If base text matches the chosen source (`shadow_state` or `analysis_snapshot`) and the incoming
ranged change set is valid, the producer MUST NOT synthesize an `edits_do_not_match_new_content`
fallback solely because local text replay used a different ordering than the parser edit chain.

If incremental parsing still fails after canonical replay normalization, the runtime may continue to
surface the existing canonical fallback reasons.

## Risks

- Sorting and normalization could accidentally change semantics for mixed full-replace/ranged
  change batches if the producer applies the rule too broadly.
- Reworking replay order could hide an existing test gap for UTF-16 to byte-offset conversion.

## Mitigations

- Restrict canonical reverse-order replay to the ranged multi-change path only.
- Add regressions that exercise multi-range edits with Cyrillic text and confirm the same normalized
  plan drives both text reconstruction and incremental parsing inputs.
