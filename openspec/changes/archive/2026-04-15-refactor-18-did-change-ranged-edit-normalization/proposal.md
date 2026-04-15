# Change: normalize ranged didChange edit replay before incremental parse

## Why

The latest incident bundle from `2026-04-12T13-15-38Z` on build `0.4.143` (`git=224c65ca`) shows
that `didChange` still fails to materialize incremental parse snapshots for valid ranged edits:

- parse-snapshot evidence reports `parseMode=full`, `baseTextSource=shadow_state`,
  `changeShape=ranged`, `fallbackReason=edits_do_not_match_new_content`;
- aggregate metrics in the same bundle show `mode_incremental=0` and a same-version full fallback.

This points to a producer-side mismatch between how the new text is reconstructed and how the edit
chain is handed to incremental parsing. As long as that mismatch remains, `refactor-16` can explain
the failure precisely, but it cannot prevent the false full-parse fallback.

## What Changes

- Require `didChange` ranged edits to be normalized into one canonical replay plan before any text
  mutation or parser-edit derivation happens.
- Require the same ordered replay plan to drive both:
  - the reconstructed `updated_text`;
  - the `parser_edits` passed into incremental parsing.
- Require multi-range `didChange` replay to preserve pre-change offsets by applying the canonical
  plan in reverse document order.
- Require valid ranged `didChange` requests to stop tripping the canonical fallback reason
  `edits_do_not_match_new_content` solely because producer replay order diverged from parser-edit
  order.

## Sequence

This change follows `refactor-16-did-change-incremental-parse-fallback-attribution` and
`refactor-17-diagnostics-save-inflight-snapshot-preference`.

It uses the new didChange-side evidence to fix the remaining producer-side defect that keeps the
exact same-version parse-snapshot path from materializing.

## Impact

- Affected specs:
  - `bsl-intellisense-v2`
- Affected code:
  - `backend/src/bin/lsp_server/server/language_server/impl_document_sync.rs`
  - `backend/src/bin/lsp_server/server/language_server/helpers.rs`
  - didChange parse-snapshot regressions and observability integration tests

## Non-Goals

- Do not redesign the canonical fallback taxonomy introduced by `refactor-16`.
- Do not change didSave follow-up branch ordering or ready-snapshot reuse.
- Do not address `bsl.getCurrentContext` churn in this change; that remains a separate follow-up
  if the bundle still shows material auxiliary pressure after this producer fix.
