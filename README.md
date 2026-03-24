# BSL Gradual Type System

*Система градуальной типизации для языка 1С:Предприятие (BSL)*

[![CI](https://github.com/defin85/bsl-gradual-types/actions/workflows/ci.yml/badge.svg)](https://github.com/defin85/bsl-gradual-types/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/rust-1.70+-brightgreen.svg)](https://www.rust-lang.org/)

`BSL Gradual Type System` — Rust workspace с runtime, LSP, web, CLI, VS Code extension и MCP server поверх общего слоя анализа BSL.

## С чего начать

- Канонический workflow для агента: `AGENTS.md`
- Curated agent-facing docs: `docs/agent/index.md`
- Developer workflow и живые команды: `docs/guides/development-workflow.md`
- Contribution и PR expectations: `CONTRIBUTING.md`

## Основные runtime surfaces

- `bsl-web-server` — web/API adapter
- `bsl-lsp-server` — LSP server
- `bsl-cli` — CLI для анализа и проверки
- `bsl-agent` — MCP stdio server с optional read-only HTTP UI
- `vscode-extension/` — VS Code extension c bundled binaries

## Быстрый старт

```bash
git clone https://github.com/defin85/bsl-gradual-types.git
cd bsl-gradual-types
cargo build --workspace
```

### Smoke по живым binary names

```bash
cargo run -p bsl-cli -- --help
cargo run -p bsl-backend --bin bsl-web-server -- --help
cargo run -p bsl-backend --bin bsl-lsp-server -- --help
cargo run -p bsl-agent -- --help
```

### Web server

```bash
cargo run -p bsl-backend --bin bsl-web-server -- \
  --port 3001 \
  --enable-cors true
```

С `syntax-helper`:

```bash
cargo run -p bsl-backend --bin bsl-web-server -- \
  --port 3001 \
  --enable-cors true \
  --syntax-helper-path examples/syntax_helper
```

### LSP server

```bash
cargo run -p bsl-backend --bin bsl-lsp-server --
```

### CLI

```bash
cargo run -p bsl-cli -- --help
cargo run -p bsl-cli -- check path/to/module.bsl
```

### MCP server

```bash
cargo run -p bsl-agent -- --help
```

Portable bootstrap и sanitized MCP examples: `docs/agent/codex-setup.md`.

## Канонический verify path

Быстрый smoke и разделение `smoke` / `manual` / `heavy` описаны в `docs/agent/verification.md`.

Локально по умолчанию:

```bash
./scripts/run-agent-readiness-checks.sh

python3 -m unittest \
  scripts/test-agent-readiness.py \
  scripts/test-intellisense-smoke-gate.py \
  scripts/test-intellisense-readiness-assets.py \
  scripts/test-ci-openspec-governance-workflow.py

./scripts/run-intellisense-tests.sh smoke
```

## CI и readiness

Workflow `CI` — активный readiness gate для репозитория. Он должен быть доступен как для автоматических `pull_request` / `push`, так и для ручного `workflow_dispatch`, а локальный verify path обязан оставаться согласованным с `docs/agent/verification.md`.

## Документация

- `docs/agent/index.md` — быстрый вход для нового агента
- `docs/README.md` — навигатор по остальной документации
- `docs/BUILD_GUIDE.md` — build/package guide
- `docs/guides/development-workflow.md` — developer commands и verify flow
- `bsl-agent/README.md` — operational details для MCP server

## VS Code extension

```bash
npm --prefix ./vscode-extension install
npm --prefix ./vscode-extension run compile:fast
npm --prefix ./vscode-extension test
```

У extension должны оставаться согласованными живые backend binaries и настройки из `vscode-extension/package.json`.
