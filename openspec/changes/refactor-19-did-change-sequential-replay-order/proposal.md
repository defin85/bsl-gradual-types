# Change: restore spec-correct sequential replay for ranged didChange incremental parsing

## Why

The live incident bundle from `2026-04-12T14-47-39Z` on build `0.4.143` (`git=3cd562d7`) still
shows `didChange` parse-snapshot fallbacks for ranged edits:

- `parseMode=full`
- `changeShape=ranged`
- `fallbackReason=edits_do_not_match_new_content`
- `changedRangesCount=0`

This means `refactor-18-did-change-ranged-edit-normalization` did not fix the real live-class.
The current implementation and its acceptance tests normalize multi-range replay in reverse
document order, but LSP `textDocument/didChange` defines `contentChanges` as sequential
state transitions that MUST be applied in receive order. As long as the producer reorders valid
receive-order edits, incremental parsing can still observe a false base/edit mismatch and force a
same-version full fallback.

## What Changes

- Replace the `refactor-18` reverse-order replay rule with spec-correct receive-order replay for
  ranged `didChange`.
- Require one canonical sequential replay plan to drive both:
  - reconstructed `updated_text`;
  - `parser_edits` passed to incremental parsing.
- Add a live-class regression for `UTF-8 BOM + CRLF` ranged edits so `conf_big`-like inputs are
  covered, not only LF-only synthetic fixtures.
- Extend bounded didChange parse-snapshot evidence so incident bundles can distinguish:
  - producer replay-order mismatches;
  - stale/incorrect base-text selection.

## Sequence

This is a correction follow-up to:

- `refactor-16-did-change-incremental-parse-fallback-attribution`
- `refactor-18-did-change-ranged-edit-normalization`

The change keeps the same incremental-parse/fallback contract, but replaces the invalid replay
ordering assumption introduced by `refactor-18`.

## Impact

- Affected specs:
  - `bsl-intellisense-v2`
- Affected code:
  - `backend/src/bin/lsp_server/server/language_server/helpers.rs`
  - `backend/src/bin/lsp_server/server/language_server/impl_document_sync.rs`
  - `backend/src/bin/lsp_server/handlers/text_document.rs`
  - `backend/src/bin/lsp_server/types.rs`
  - `backend/src/bin/lsp_server/server/core/tests.rs`
  - `bsl-runtime/src/system/parser_coordinator/tests.rs`

## Non-Goals

- Do not redesign the canonical fallback taxonomy from `refactor-16`.
- Do not change `didSave` follow-up branch ordering or same-version readiness gating.
- Do not address `bsl.getCurrentContext` churn in this change.
