# Agent Architecture Map

## Workspace Map

- `backend/`
  HTTP + LSP adapter crate. Живые бинарные surfaces: `bsl-web-server`, `bsl-lsp-server`.
- `bsl-agent/`
  MCP stdio adapter с optional read-only HTTP UI. Живой бинарный surface: `bsl-agent`.
- `cli/`
  Командная строка поверх runtime. Живой бинарный surface: `bsl-cli`.
- `bsl-runtime/`
  Общий runtime слой: startup, deps/cache wiring, application services.
- `analysis-v2/`, `semantic-diagnostics/`, `syntax/`, `shared/`
  Основные analysis/domain/parsing слои.
- `frontend/`
  Leptos/WASM frontend и assets для `target/site/`.
- `vscode-extension/`
  VS Code extension, которая бандлит Rust binaries и использует LSP/webviews.
- `scripts/`
  Repo policies, smoke/perf/readiness gates, docs checks.
- `openspec/`
  Intent contract: proposals, tasks, specs, validation evidence.
- `.beads/`
  Live task graph для execution stage.

## Binary Surfaces

- `bsl-web-server`
  Web API + static SPA. Entry point: `backend/src/main.rs`.
- `bsl-lsp-server`
  STDIO LSP server. Entry point: `backend/src/bin/lsp_server/main.rs`.
- `bsl-cli`
  CLI subcommands `analyze`, `check`, `complete`, `info`, `analyze-ir`, `cache`. Entry point: `cli/src/main.rs`.
- `bsl-agent`
  MCP stdio server + `ui` subcommands. Entry point: `bsl-agent/src/main.rs`.

## Important Boundaries

- `bsl-backend` зависит от `bsl-runtime`, а не наоборот.
- `bsl-agent` MUST NOT depend on `bsl-backend`.
- Canonical instruction layer идёт от root `AGENTS.md`, а не от `.mcp.json` или ad-hoc README.

## Where To Read Next

- Backend/LSP/web adapter map: `backend/README.md`
- MCP runtime details: `bsl-agent/README.md`
- Workspace conventions и process contract: `AGENTS.md`, `openspec/project.md`, `openspec/AGENTS.md`
- Build/test commands: `docs/agent/verification.md`

## High-Friction Entry Points

- LSP server runtime and handlers:
  `backend/src/bin/lsp_server/`
- Web server startup:
  `backend/src/main.rs`
- CLI command surface:
  `cli/src/args.rs`, `cli/src/main.rs`
- MCP session/server wiring:
  `bsl-agent/src/main.rs`, `bsl-agent/src/server/`, `bsl-agent/src/session/`
- VS Code extension scripts and tests:
  `vscode-extension/package.json`, `vscode-extension/src/lsp/`, `vscode-extension/src/test/`
