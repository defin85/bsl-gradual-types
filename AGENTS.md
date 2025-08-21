# Repository Guidelines

## Project Structure & Module Organization
- `src/`: Rust library crates split into `core/`, `parser/`, `adapters/`, `query/`, `ideal/`, plus binaries in `src/bin/` (e.g., `type_check.rs`, `lsp_server.rs`, `bsl-web-server.rs`).
- `tests/`: Integration tests (`*_test.rs`), fixtures in `tests/fixtures/`, sample XML in `tests/test_xml/`.
- `examples/`: Runnable examples (e.g., `query_demo.rs`).
- `benches/`: Criterion benchmarks (e.g., `syntax_helper_parser_bench.rs`).
- `docs/`: Architecture, design, and development notes.
- `vscode-extension/`: VSCode client extension (TypeScript).

## Build, Test, and Development Commands
- `cargo build [--release]`: Build all crates and binaries.
- `cargo test`: Run Rust unit and integration tests.
- `cargo run --bin lsp-server`: Start the LSP server for editors.
- `cargo run --bin bsl-web-server -- --port 8080`: Run the web UI/API.
- `cargo run --bin type-check -- --file module.bsl [--guided --config path]`: CLI type checking.
- `cargo bench`: Run Criterion benchmarks in `benches/`.
- `cd vscode-extension && npm install && npm test`: Extension setup and tests.

## Coding Style & Naming Conventions
- Formatter: `cargo fmt` (Rust 2021, 4‑space indent). Linting: `cargo clippy -- -D warnings`.
- Naming: modules/files `snake_case`, types/traits `CamelCase`, constants `SCREAMING_SNAKE_CASE`, functions/vars `snake_case`.
- Layout: public modules declared in `src/lib.rs`; binaries live in `src/bin/` and are referenced in `Cargo.toml` `[[bin]]`.
- Keep functions small, prefer explicit types in public APIs, add Rustdoc `///` with examples for exported items.

## Testing Guidelines
- Frameworks: Rust test harness with `pretty_assertions` and `insta` for snapshots (where applicable).
- Conventions: integration tests under `tests/` named `*_test.rs`; unit tests in-module behind `#[cfg(test)]`.
- Run: `cargo test` (Rust) and `cd vscode-extension && npm test` (extension).
- Add tests for all new behavior; prefer deterministic fixtures (`tests/fixtures/`, `tests/test_xml/`).

## Commit & Pull Request Guidelines
- Commits follow Conventional style: `type(scope): short description`. Types: `feat`, `fix`, `docs`, `style`, `refactor`, `test`, `chore`. Scopes: `core`, `parser`, `lsp`, `cli`, `vscode`, `web`, `docs`.
- Branches: `feature/<name>`, `fix/<name>`.
- PRs: include description, rationale, and linked issue (`Fixes #123`); add tests/docs; attach screenshots/recordings for `vscode-extension` or web changes.
- Before opening: run `cargo fmt`, `cargo clippy -- -D warnings`, `cargo test`, and extension tests if touched.

## Security & Configuration Tips
- Do not commit proprietary 1C configurations; use sanitized samples from `tests/test_xml/` or synthetic fixtures.
- Avoid committing packaged artifacts (`*.vsix`, `target/`); keep secrets and local paths out of code and docs.

## Коммуникация
- Язык: Всегда общаться на русском языке во всех каналах (issues, обсуждения, PR-описания, комментарии к коду и документации).
