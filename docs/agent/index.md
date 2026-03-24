# Agent Docs Index

Этот раздел собирает короткий, проверяемый путь входа в репозиторий для Codex-агента.

## С чего начинать

1. Канонический policy/source-of-truth по инструкциям: `AGENTS.md` в корне репозитория.
2. Curated карта репозитория: `docs/agent/architecture-map.md`.
3. Run/test/verify contract: `docs/agent/verification.md`.
4. Трассировка артефактов OpenSpec/Beads/CI/runtime: `docs/agent/task-artifacts.md`.

## Source Of Truth Order

- `AGENTS.md` в корне задаёт глобальные правила, workflow и handoff contract.
- `docs/agent/*` даёт agent-facing навигацию и runbook, но не заменяет root policy.
- Локальные `AGENTS.md` в подпроектах добавляют только area-specific контекст.
- `AGENTS.override.md` допустим только для intentional override с явным reason.

## Короткая Карта

- Workspace crates и бинарные entry points: `docs/agent/architecture-map.md`
- Канонические команды и expected outcomes: `docs/agent/verification.md`
- OpenSpec change, Beads backlog, CI jobs и evidence: `docs/agent/task-artifacts.md`

## Быстрые Ответы

- Что это за проект: Rust workspace для CLI, web, LSP и MCP поверх BSL gradual typing.
- Где смотреть entry points: `backend/src/main.rs`, `backend/src/bin/lsp_server/main.rs`, `cli/src/main.rs`, `bsl-agent/src/main.rs`.
- Где запускать smoke path: `docs/agent/verification.md`.
- Как не потерять traceability: `docs/agent/task-artifacts.md`.

## Durable Doc Policy

- По умолчанию используй path/section links, чтобы документация переживала перестановку строк.
- Line-level links допустимы только в evidence, review notes и generated references.
- Для agent-facing docs фиксируй только живые binary names, существующие пути и проверяемые команды.
