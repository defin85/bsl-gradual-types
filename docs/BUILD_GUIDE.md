# Build Guide

Канонический build/test/runbook для агента находится в `docs/agent/verification.md`. Этот файл концентрируется на build/package paths и итоговых артефактах.

## Workspace build

```bash
cargo build --workspace
cargo build --release --workspace
```

## Выборочные бинарные сборки

```bash
cargo build -p bsl-backend --bin bsl-web-server --release
cargo build -p bsl-backend --bin bsl-lsp-server --release
cargo build -p bsl-cli --release
cargo build -p bsl-agent --release
```

Ожидаемые release binaries:

- `target/release/bsl-web-server`
- `target/release/bsl-lsp-server`
- `target/release/bsl-cli`
- `target/release/bsl-agent`

## Frontend / WASM

```bash
(cd frontend && NO_COLOR=true trunk build --release)
```

Output:

- `target/site/` — static assets for web UI

## VS Code extension package

```bash
npm --prefix ./vscode-extension install
npm --prefix ./vscode-extension run package
```

Это собирает extension bundle, копирует bundled binaries и подготавливает `.vsix` packaging flow.

## Smoke после сборки

```bash
cargo run -p bsl-cli -- --help
cargo run -p bsl-backend --bin bsl-web-server -- --help
cargo run -p bsl-backend --bin bsl-lsp-server -- --help
cargo run -p bsl-agent -- --help
```

## Связанные документы

- `agent/verification.md` — canonical verify path
- `guides/development-workflow.md` — developer workflow
- `../bsl-agent/README.md` — operational details для MCP server
