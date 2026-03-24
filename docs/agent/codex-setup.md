# Codex Setup

Root `AGENTS.md` остаётся каноническим policy source. Этот документ описывает только portable bootstrap для локального Codex/MCP окружения.

## Принципы

- Checked-in `.mcp.json` — example-only, а не канонический onboarding path.
- Machine-specific absolute paths и секреты не должны попадать в репозиторий.
- Повторяющиеся workflow должны выноситься в repo-local skills под `.agents/skills/`.

## Минимальные prerequisites

- `cargo`
- `python3`
- `npm` для `vscode-extension/`
- `trunk` только для frontend/WASM сценариев

## Минимальный MCP bootstrap

Checked-in `.mcp.json` использует portable launch через `cargo run -p bsl-agent --`.

Runtime smoke:

```bash
cargo run -p bsl-agent -- --help
```

Если нужен read-only HTTP UI:

```bash
BSL_AGENT_HTTP_ADDR=127.0.0.1:0 cargo run -p bsl-agent -- --help
```

## Local overrides

- Локальные absolute paths, extra MCP servers и секреты держите в личной конфигурации, а не в tracked `.mcp.json`.
- Для operational details по `bsl-agent` используйте `../../bsl-agent/README.md`.
- Для smoke/manual/heavy verify path используйте `verification.md`.

## Recurring workflows

Repo-local skills должны покрывать как минимум:

- workspace verification
- `bsl-agent` MCP bootstrap/smoke
- OpenSpec delivery matrix
- docs drift audit
