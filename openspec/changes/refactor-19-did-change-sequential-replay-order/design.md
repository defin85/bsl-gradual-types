## Context

`didChange` ranged edit handling spans:

- LSP adapter replay planning
- local text reconstruction for shadow-state updates
- incremental parse handoff into `parser_coordinator`
- observability evidence exported into incident bundles

`refactor-18` standardized these paths around one replay plan, but it encoded the wrong ordering
model for LSP batched ranged changes. The result is that synthetic reverse-order fixtures pass
while live `conf_big` traces still fall back with `edits_do_not_match_new_content`.

## Goals / Non-Goals

- Goals:
  - align ranged `didChange` replay with LSP sequential change semantics
  - keep one canonical replay plan shared by text reconstruction and parser edits
  - reproduce the live class with `BOM + CRLF`
  - improve bounded evidence so future bundles show whether the miss came from replay order or
    from wrong base-text selection
- Non-goals:
  - no new parse fallback reason taxonomy
  - no redesign of `didSave` follow-up
  - no performance tuning for `bsl.getCurrentContext`

## Design

### 1. Canonical replay plan becomes sequential, not reverse-sorted

For ranged `TextDocumentContentChangeEvent[]`, the canonical plan MUST preserve receive order.
Each change in the batch is interpreted as operating on the document state produced by the
previous change in the same notification.

The producer still uses one canonical plan for both:

- `updated_text` reconstruction
- `parser_edits`

but the plan is now defined as "LSP receive order", not "reverse document order".

### 2. Live-class regression must cover BOM + CRLF

The current synthetic coverage is too weak because it uses LF-only fixtures and ranges that remain
valid under a reverse-order model. A dedicated regression should seed a file with:

- `UTF-8 BOM`
- `CRLF`
- multiple ranged edits in one `didChange`

and compute the second edit range against the intermediate state after the first edit, as the LSP
spec requires.

### 3. Bounded evidence needs one more layer of producer attribution

Current didChange parse-snapshot evidence carries:

- `parseMode`
- `baseTextSource`
- `changeShape`
- `changedRangesCount`
- `fallbackReason`

This is not enough to distinguish:

- wrong replay order on a valid batch
- stale or mismatched base text before replay starts

The change should extend version-bound evidence with low-cardinality fields:

- `contentChangesCount`
- `replayOrder` with canonical value `receive_order`
- `baseDocumentVersion` when known from shadow state, otherwise omitted

No raw text, raw ranges, or high-cardinality payloads are needed.

## Risks

- Existing tests from `refactor-18` that encoded reverse-order semantics will need to be replaced,
  not merely updated, or they will keep asserting the wrong contract.
- If the live fallback is not replay-order-related after all, the new bounded evidence will still
  narrow the remaining search to base-text drift instead of replay drift.
