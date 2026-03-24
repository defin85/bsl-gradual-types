# Development Workflow Guide

Этот документ фиксирует текущие команды разработки. Канонический agent-facing smoke/manual split находится в `../agent/verification.md`; здесь он продублирован только в объёме, необходимом для разработчика.

## Workspace build

```bash
cargo build --workspace
cargo build --release --workspace
cargo build --profile dev-fast
```

## Живые runtime surfaces

```bash
cargo run -p bsl-cli -- --help
cargo run -p bsl-backend --bin bsl-web-server -- --help
cargo run -p bsl-backend --bin bsl-lsp-server -- --help
cargo run -p bsl-agent -- --help
```

Expected outcome:

- все команды завершаются с кодом `0`
- help output использует текущие names `bsl-cli`, `bsl-web-server`, `bsl-lsp-server`, `bsl-agent`

## Default smoke path

```bash
./scripts/run-agent-readiness-checks.sh

python3 -m unittest \
  scripts/test-agent-readiness.py \
  scripts/test-intellisense-smoke-gate.py \
  scripts/test-intellisense-readiness-assets.py \
  scripts/test-ci-openspec-governance-workflow.py

./scripts/run-intellisense-tests.sh smoke
```

## Rust verification

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
```

Для более узкой локальной проверки:

```bash
cargo test -p bsl-backend
cargo test -p bsl-agent
cargo test -p bsl-cli
```

## Web server

```bash
cargo run -p bsl-backend --bin bsl-web-server -- \
  --port 3002 \
  --enable-cors true
```

С Syntax Helper:

```bash
cargo run -p bsl-backend --bin bsl-web-server -- \
  --port 3002 \
  --enable-cors true \
  --syntax-helper-path examples/syntax_helper
```

## LSP server

```bash
cargo run -p bsl-backend --bin bsl-lsp-server --
RUST_LOG=debug cargo run -p bsl-backend --bin bsl-lsp-server --
cargo run --release -p bsl-backend --bin bsl-lsp-server --
```

## CLI

```bash
cargo run -p bsl-cli -- --help
cargo run -p bsl-cli -- check path/to/module.bsl
cargo run -p bsl-cli -- info "Справочники.Номенклатура"
cargo run -p bsl-cli -- complete "Справочники."
```

## MCP server

```bash
cargo run -p bsl-agent -- --help
cargo run -p bsl-agent -- ui list --help
cargo run -p bsl-agent -- ui url --help
```

Portable setup examples: `../agent/codex-setup.md`.

## VS Code extension

```bash
npm --prefix ./vscode-extension install
npm --prefix ./vscode-extension run compile:fast
npm --prefix ./vscode-extension run lint
npm --prefix ./vscode-extension test
```

## Frontend / WASM

```bash
(cd frontend && NO_COLOR=true trunk build --release)
```

## Heavy / readiness gates

```bash
./scripts/run-intellisense-perf.sh
./scripts/validate-v2-completion-gates.sh
```

Запускайте их только когда change действительно требует readiness/perf evidence.
