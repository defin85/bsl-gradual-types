# bsl-backend

HTTP + LSP adapter crate for BSL Gradual Types.

## Crate boundaries

- Core runtime logic (startup/deps/cache wiring, parsing/data helpers, type-system services) lives in `bsl-runtime/`.
- `bsl-backend` is a thin adapter layer (web/LSP wiring + presentation helpers) that depends on `bsl-runtime`.
- `bsl-agent` MUST NOT depend on `bsl-backend`.

## Code layout

- Library entry: `backend/src/lib.rs`
- HTTP/LSP binaries: `backend/src/bin/*`

