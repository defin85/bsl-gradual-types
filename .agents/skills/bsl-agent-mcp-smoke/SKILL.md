---
name: bsl-agent-mcp-smoke
description: Run smoke checks for changes touching `bsl-agent`, MCP bootstrap, `.mcp.json`, or the read-only HTTP UI. Use to verify CLI help, UI helper commands, and bootstrap/docs alignment.
---

# BSL Agent MCP Smoke

Используй этот skill, когда change затрагивает `bsl-agent`, MCP bootstrap, `.mcp.json` или read-only HTTP UI.

## Steps

1. Сверь root policy в `AGENTS.md` и bootstrap notes в `docs/agent/codex-setup.md`.
2. Запусти `cargo run -p bsl-agent -- --help`.
3. При необходимости проверь UI helpers:
   `cargo run -p bsl-agent -- ui list --help`
   `cargo run -p bsl-agent -- ui url --help`
4. Если change затрагивает logging/bootstrap, используй smoke commands из `bsl-agent/README.md`.

## Expected Outcome

- portable bootstrap остаётся без machine-specific tracked config
- `bsl-agent` help/UI surfaces не расходятся с docs
