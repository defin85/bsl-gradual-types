# lsp-completion-timeline v1

## 1.0.0

- Initial baseline for server-driven completion timeline contract.
- Fixes transport path: `workspace/executeCommand` with command `bsl.getCompletionTimeline`.
- Fixes response envelope version (`version=1`) and bounded retention default (`max_entries=200`).
- Fixes canonical terminal supersession outcome to `superseded` (public API value).
- Migration note: initial release, no migration required.
