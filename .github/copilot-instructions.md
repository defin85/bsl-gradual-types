# Repository Guidelines

## Project Overview
BSL Gradual Type System is an enterprise-ready gradual typing system for the 1C:Enterprise BSL language. It combines static analysis and runtime contracts to improve code quality, developer productivity, and scalability.

## Project Structure
- **src/**: Core Rust library crates, including `core/`, `parser/`, `adapters/`, `query/`, `ideal/`. Binaries are in `src/bin/` (e.g., `type_check.rs`, `lsp_server.rs`, `bsl-web-server.rs`).
- **tests/**: Integration tests (`*_test.rs`), fixtures in `tests/fixtures/`, and sample XML in `tests/test_xml/`.
- **examples/**: Runnable examples demonstrating key features (e.g., `query_demo.rs`).
- **benches/**: Criterion benchmarks for performance testing (e.g., `syntax_helper_parser_bench.rs`).
- **docs/**: Architecture, design, and development notes.
- **vscode-extension/**: TypeScript-based VSCode client extension.

## Build, Test, and Development Commands
- **Build**:
  - `cargo build [--release]`: Build all crates and binaries.
  - `cargo check`: Verify code without building.
- **Test**:
  - `cargo test`: Run Rust unit and integration tests.
  - `cd vscode-extension && npm test`: Run extension tests.
- **Run**:
  - `cargo run --bin lsp-server`: Start the LSP server.
  - `cargo run --bin bsl-web-server -- --port 8080`: Launch the web UI/API.
  - `cargo run --bin type-check -- --file module.bsl [--guided --config path]`: CLI type checking.
- **Benchmarks**:
  - `cargo bench`: Run Criterion benchmarks.

## Coding Style & Conventions
- **Formatting**: Use `cargo fmt` (Rust 2021, 4-space indent).
- **Linting**: Run `cargo clippy -- -D warnings`.
- **Naming**:
  - Modules/files: `snake_case`
  - Types/traits: `CamelCase`
  - Constants: `SCREAMING_SNAKE_CASE`
  - Functions/variables: `snake_case`
- **Documentation**: Add Rustdoc `///` with examples for exported items.
- **Structure**: Public modules declared in `src/lib.rs`; binaries in `src/bin/`.

## Testing Guidelines
- **Frameworks**: Use Rust test harness with `pretty_assertions` and `insta` for snapshots.
- **Conventions**: Integration tests in `tests/` named `*_test.rs`; unit tests in-module behind `#[cfg(test)]`.
- **Fixtures**: Prefer deterministic fixtures in `tests/fixtures/` and `tests/test_xml/`.

## Commit & Pull Request Guidelines
- **Commits**: Follow Conventional style (`type(scope): short description`). Types: `feat`, `fix`, `docs`, `style`, `refactor`, `test`, `chore`. Scopes: `core`, `parser`, `lsp`, `cli`, `vscode`, `web`, `docs`.
- **Branches**: Use `feature/<name>` or `fix/<name>`.
- **Pull Requests**:
  - Include description, rationale, and linked issue (`Fixes #123`).
  - Add tests/docs.
  - Attach screenshots/recordings for `vscode-extension` or web changes.
- **Pre-submit checks**: Run `cargo fmt`, `cargo clippy -- -D warnings`, `cargo test`, and extension tests if touched.

## Security & Configuration Tips
- Avoid committing proprietary 1C configurations; use sanitized samples from `tests/test_xml/` or synthetic fixtures.
- Do not commit packaged artifacts (`*.vsix`, `target/`); keep secrets and local paths out of code and docs.

## Communication
- Language: Always communicate in Russian across all channels (issues, discussions, PR descriptions, code comments, and documentation).

## Integration Points
- **LSP Server**: Provides type hints, completions, and diagnostics.
- **VSCode Extension**: Enhances developer experience with type hints and code actions.
- **Web Interface**: Visualizes type information and analysis results.
- **CLI Tools**: Automates type checking and profiling.

## Examples of Patterns
- **Flow-Sensitive Analysis**: Tracks type changes during execution.
- **Union Types**: Supports weighted union types (e.g., `String 60% | Number 40%`).
- **Type Narrowing**: Refines types in conditional statements (e.g., `ТипЗнч(x) = Тип("Строка")`).
