# Backend Agent Notes

Эти инструкции дополняют root `AGENTS.md` и `docs/agent/*`.

## Scope

- `backend/` — HTTP/LSP adapter crate `bsl-backend`
- Связанные entry points: `backend/src/main.rs`, `backend/src/bin/lsp_server/main.rs`
- Core runtime logic живёт в `bsl-runtime/`; не тащи adapter-specific код обратно в runtime без явной причины

## Main Entry Points

- Web binary: `cargo run -p bsl-backend --bin bsl-web-server -- --help`
- LSP binary: `cargo run -p bsl-backend --bin bsl-lsp-server -- --help`
- Library boundary: `backend/src/lib.rs`

## Local Verify

- Минимальный smoke для CLI contracts:

```bash
cargo run -p bsl-backend --bin bsl-web-server -- --help
cargo run -p bsl-backend --bin bsl-lsp-server -- --help
```

- Если меняется backend behavior, предпочитай узкий `cargo test -p bsl-backend ...`, затем общий путь из `docs/agent/verification.md`.

## Important Files

- `backend/Cargo.toml` — package/bin names
- `backend/src/main.rs` — web startup
- `backend/src/bin/lsp_server/main.rs` — LSP startup
- `backend/README.md` — crate boundary contract

## Boundaries

- `bsl-backend` остаётся thin adapter layer над `bsl-runtime`.
- Не добавляй зависимость `bsl-agent -> bsl-backend`; этот контракт проверяется отдельно.
