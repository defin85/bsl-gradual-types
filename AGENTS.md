<!-- OPENSPEC:START -->

# OpenSpec Instructions

These instructions are for AI assistants working in this project.

Always open `@/openspec/AGENTS.md` when the request:

- Mentions planning or proposals (words like proposal, spec, change, plan)
- Introduces new capabilities, breaking changes, architecture shifts, or big performance/security work
- Sounds ambiguous and you need the authoritative spec before coding

Use `@/openspec/AGENTS.md` to learn:

- How to create and apply change proposals
- Spec format and conventions
- Project structure and guidelines

Keep this managed block so 'openspec update' can refresh the instructions.

<!-- OPENSPEC:END -->

# Root Policy

`AGENTS.md` в корне репозитория — канонический source of truth для agent instructions.

## Source Of Truth Order

1. Этот файл задаёт глобальные правила, OpenSpec/Beads workflow и handoff contract.
2. `docs/agent/index.md` и связанные `docs/agent/*` дают curated navigation, run/test/runbook и traceability.
3. Локальные `AGENTS.md` в подпроектах добавляют только area-specific контекст.
4. `AGENTS.override.md` допустим только как intentional override с явным reason.

## Language

- Планы, спеки и описания change ведём на русском языке.
- Общепринятые technical terms, API names и code identifiers можно оставлять на английском.

## Execution Workflow

We operate in a cycle: **OpenSpec (What) -> Beads (How) -> Code (Implementation)**.

### Approval Gate

- До явного approval (`Go!` или `/openspec-to-beads <change-id>`) меняются только OpenSpec artifacts.
- После approval change должен быть отражён в Beads graph.
- Если `/openspec-to-beads <change-id>` недоступен, агент вручную создаёт эквивалентный epic/tasks/dependencies в `bd`.

### Execution Loop

- Перед кодом читай `proposal.md`, `tasks.md`, `design.md` (если есть), `specs/**/spec.md`.
- Для code changes работай только из `bd ready`.
- Держи OpenSpec intent, Beads graph и repository state синхронными.
- Для каждого mandatory requirement нужна traceability: `Requirement -> Code -> Test`.

### Delivery Contract

- Mandatory requirements нельзя отдавать как `partially implemented`.
- Если requirement сейчас недоставим, остановись и зафиксируй blocker.
- Финальный отчёт должен ссылаться на concrete files/tests.

## Curated Agent Docs

Начинай agent-facing discovery отсюда:

- `docs/agent/index.md` — стартовый индекс.
- `docs/agent/architecture-map.md` — workspace map и entry points.
- `docs/agent/verification.md` — canonical run/test/verify contract.
- `docs/agent/task-artifacts.md` — OpenSpec/Beads/CI/runtime traceability.
- `docs/agent/codex-setup.md` — portable Codex/MCP bootstrap и sanitized examples.

## Search And Verification

- Search order: `mcp__claude-context__search_code` -> `rg` -> `rg --files`.
- Подтверждай важные выводы как минимум двумя источниками: code + test/spec/doc.
- После изменений запускай минимальный релевантный verify set, затем более широкий gate при необходимости.
- Канонические команды и expected outcomes описаны в `docs/agent/verification.md`.

## Local Instruction Zones

- `backend/AGENTS.md` — backend, web, LSP, Rust-specific verify flow.
- `bsl-agent/AGENTS.md` — MCP bootstrap, smoke path, runtime artifacts.
- `vscode-extension/AGENTS.md` — Node tooling, extension tests, VS Code packaging flow.

## Durable Doc Policy

- Durable docs по умолчанию используют path/section links.
- Line-level links допустимы только для evidence, review notes и generated references.
- Agent-facing docs должны ссылаться только на живые команды, пути и binary names.

## Session Completion

- Work session не завершён, пока `git push` не прошёл успешно.
- Перед handoff: закрой или обнови Beads tasks, прогоняй проверки, затем `git pull --rebase`, `bd vc status`, при необходимости `bd vc commit`, `git push`.
- Если push блокирован внешним ограничением, явно укажи blocker.
