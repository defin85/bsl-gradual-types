# Change: Add BSL getAllTypes mode to Zed Symbol Browser

## Why
The generic Zed Symbol Browser now works through `workspace/symbol`, but that surface only exposes routine-like symbols and cannot show the BSL TypeRepository tree that users expect from the existing VS Code Type Repository panel.

BSL projects need a dedicated Symbol Browser mode backed by the BSL language server TypeRepository contract while preserving the generic `workspace/symbol` behavior for Rust and other languages.

## What Changes
- Add a BSL-specific data source to the Zed `symbol_browser` crate.
- Detect BSL language servers that expose the existing `bsl.getAllTypes` command through `workspace/executeCommand`.
- Fetch bounded paged `getAllTypes` results from every capable open-worktree server and render BSL groups by item-level source/category instead of generic LSP `SymbolKind`.
- Preserve the generic `workspace/symbol` mode as the default fallback for non-BSL projects and unsupported BSL servers.
- Document and test the BSL LSP `getAllTypes` response contract used by Zed.

## Impact
- Affected specs: `zed-symbol-browser`, `bsl-intellisense`
- Affected code in Zed fork: `crates/symbol_browser`, `crates/project` or a narrow adjacent LSP helper
- Affected code in this repo: BSL LSP `bsl.getAllTypes` contract tests; a direct `bsl/getAllTypes` compatibility alias remains deferred optional work unless separately approved
- Requires the baseline `add-zed-symbol-browser-panel` change to exist in the Zed fork

## Non-Goals
- Replacing generic `workspace/symbol` for non-BSL languages
- Interactive navigation/click-to-open behavior
- Full TypeRepository details UI for methods/properties beyond an initial grouped type list
- Making Zed extensions create panels without the forked core UI change
