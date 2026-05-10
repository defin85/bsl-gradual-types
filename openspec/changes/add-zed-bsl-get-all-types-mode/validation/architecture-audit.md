## Locked decisions

1. Initial Zed implementation uses standard LSP `workspace/executeCommand` with command `bsl.getAllTypes`.
2. Direct `bsl/getAllTypes` is deferred optional compatibility work unless explicitly approved and tested.
3. BSL mode is capability-gated by advertised `bsl.getAllTypes`; unsupported servers use existing generic `workspace/symbol`.
4. BSL mode queries every capable open-worktree language server and merges results deterministically.
5. Pagination is bounded by checked-in page-size, max-page, and max-item constants.
6. Runtime getAllTypes page failure after BSL mode selection is a visible BSL error, not a silent generic fallback.
7. BSL groups are built from item-level `source` and `category`, not from response-level `categories`.
8. LSP transport and DTO parsing live in Zed project-layer code; `symbol_browser` consumes a UI-neutral model.

## Audit matrix

| Area | Finding | Plan action |
|------|---------|-------------|
| LSP transport | Backend currently exposes dot command `bsl.getAllTypes` through `workspace/executeCommand`; slash alias is not checked-in behavior. | Use executeCommand first; keep alias deferred unless separately approved and tested. |
| Capability gate | Zed generic symbol path already checks language-server capabilities before `workspace/symbol`; BSL mode needs the same fail-closed pattern. | Gate on advertised `execute_command_provider.commands` containing `bsl.getAllTypes`. |
| Multi-root workspace | A single "active server" wording could miss BSL worktrees or make behavior non-deterministic. | Query every capable open-worktree server and sort/merge deterministically. |
| Pagination | Backend supports `limit`/`offset`; an unbounded panel fetch would risk UI responsiveness. | Require visible constants and tests for stop conditions and cap path. |
| Grouping | Current backend `categories` is response metadata; item entries carry the real `source`/`category`. | Specify item-level fields as source of truth for groups. |
| Error semantics | Generic fallback is correct for unsupported capability, but wrong for runtime failure after BSL mode selected. | Preserve fallback only for unsupported selection; show visible BSL error for runtime failure. |
| Zed layering | Putting executeCommand DTO parsing in `symbol_browser` would couple UI to LSP transport. | Keep transport/DTO/pagination in `project` or narrow project-layer helper. |

## Execution plan

1. Backend: add focused coverage for executeCommand advertisement, request shape, pagination, category filtering, and empty-domain response.
2. Zed project layer: add typed DTOs, capable-server discovery, bounded paged fetch, deterministic multi-server merge, and error propagation.
3. Symbol Browser: add data-source mode selection, render BSL source/category groups from item-level fields, and preserve generic SymbolKind path unchanged.
4. Verification: run backend focused tests, Zed project/symbol_browser tests, `cargo check -p zed`, BSL live-smoke with the dev extension installed, Rust generic live-smoke, and strict OpenSpec validation.

## Exact wording fixes applied

- Replaced initial `SHOULD` transport wording with `MUST workspace/executeCommand bsl.getAllTypes`.
- Deferred slash alias instead of leaving it as an implementation-time decision.
- Added explicit multi-server selection and deterministic merge requirements.
- Added checked-in pagination cap requirements and cap-path tests.
- Added item-level `source`/`category` grouping requirements.
- Added Zed crate-boundary requirement for project-layer LSP handling.

## Remaining assumptions

- The checked-in Zed fork remains available under `/home/egor/code/zed`.
- The `add-zed-symbol-browser-panel` baseline is installed or buildable before this implementation starts.
- Live BSL verification requires the dev Zed extension installed into the test Zed user-data directory, not merely checked-in extension source.
