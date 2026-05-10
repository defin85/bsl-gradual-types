## Implementation evidence

Status after implementation pass: code, focused tests, Zed compile check, and strict OpenSpec validation passed. GUI live-smoke remains open.

## Backend

- Added focused handler coverage for `bsl.getAllTypes` pagination, category filtering, item-level `source`/`category`, and empty-domain response.
- Added initialization coverage that `execute_command_provider.commands` advertises `bsl.getAllTypes`.
- Direct `bsl/getAllTypes` alias remains intentionally deferred.

Validation:

```bash
cargo test -p bsl-backend --bin bsl-lsp-server get_all_types -- --nocapture
cargo test -p bsl-backend --bin bsl-lsp-server p9a_formatting_disabled_does_not_advertise_capability_and_returns_null -- --nocapture
```

## Zed fork

- Added `project::BslTypeRepositorySymbol` and `Project::bsl_type_repository_symbols`.
- Added project-layer `workspace/executeCommand` fetch for `bsl.getAllTypes` with checked-in page-size, max-page, and max-item caps.
- Capability detection is command-specific: no advertised `bsl.getAllTypes` returns unsupported (`Ok(None)`) so Symbol Browser keeps generic `workspace/symbol`.
- Runtime getAllTypes failure returns an error; Symbol Browser shows the existing `Symbols unavailable` state instead of falling back to generic symbols.
- Symbol Browser groups BSL entries by item-level `source` and `category`; generic `SymbolKind` grouping remains for fallback mode.

Validation:

```bash
cargo test -p project --features test-support test_bsl_get_all_types -- --nocapture
cargo test -p project --features test-support test_bsl_type_repository_symbols -- --nocapture
cargo test -p symbol_browser -- --nocapture
cargo check -p zed
```

## Open verification

- BSL X11 live-smoke with installed dev extension is not yet run.
- Rust X11 live-smoke for generic `workspace/symbol` is not yet run.

Those two tasks remain unchecked in `tasks.md` and must not be claimed as complete until real Zed GUI evidence exists.
