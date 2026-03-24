# BSL Agent Notes

Эти инструкции дополняют root `AGENTS.md` и `docs/agent/*`.

## Scope

- `bsl-agent/` — MCP stdio server и optional read-only HTTP UI
- Основной entry point: `bsl-agent/src/main.rs`
- Operational contract и env surface описаны в `bsl-agent/README.md`

## Main Entry Points

- Binary help:

```bash
cargo run -p bsl-agent -- --help
```

- UI helper commands:

```bash
cargo run -p bsl-agent -- ui list --help
cargo run -p bsl-agent -- ui url --help
```

## Local Verify

- Минимум: `cargo run -p bsl-agent -- --help`
- Если затронут MCP/bootstrap/logging flow, сверяй примеры и smoke path с `docs/agent/verification.md` и `bsl-agent/README.md`

## Runtime Artifacts

- File log precedence и state/cache paths описаны в `bsl-agent/README.md`
- Checked-in `.mcp.json` не каноничен; portable bootstrap должен ссылаться на `docs/agent/codex-setup.md`

## Boundaries

- `bsl-agent` — adapter layer над `bsl-runtime`
- Не добавляй зависимость на `bsl-backend`
- HTTP UI остаётся read-only
