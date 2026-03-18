# Change: add-codex-agent-readiness-workflow

## Why
Read-only аудит репозитория показал, что новый Codex-агент не получает один короткий и надёжный путь входа в проект.

Основные проблемы лежат не в коде анализатора, а в agent-facing слое репозитория:
- корневой `AGENTS.md` перегружен процессом и search tips, но не работает как короткий project map;
- первичные onboarding-доки содержат устаревшие команды и пути, из-за чего агент вынужден перепроверять README через терминал;
- run/test/verify path не сведён в один исполнимый контракт;
- portable Codex/MCP bootstrap не оформлен как канонический артефакт;
- повторяющиеся workflow для Codex не вынесены в repo-local skills;
- freshness checks частично есть, но не закрывают drift именно в agent-facing документации и командах.

В результате новый агент тратит время на археологию вместо выполнения задачи, а длинные сессии становятся менее надёжными из-за слабого “working state”.

## What Changes
- Зафиксировать в `dev-workflow` обязательный curated agent-facing documentation surface для Codex под `docs/agent/`:
  - `index.md`
  - `architecture-map.md`
  - `verification.md`
  - `task-artifacts.md`
- Зафиксировать layered AGENTS contract:
  - корневой `AGENTS.md` как короткий dispatcher/index;
  - локальные `AGENTS.md` в high-friction зонах (`backend/`, `bsl-agent/`, `vscode-extension/`);
  - `AGENTS.override.md` только как исключение для реальной замены родительских инструкций.
- Зафиксировать канонический agent verification/runbook contract:
  - живые команды для `bsl-cli`, `bsl-web-server`, `bsl-lsp-server`, `bsl-agent`;
  - разделение smoke/manual/heavy;
  - явные prerequisites и expected outcomes.
- Зафиксировать portable Codex setup и repo-local skills:
  - sanitized examples для Codex/MCP bootstrap без machine-specific путей и секретов;
  - минимальный набор repo-local skills для recurring workflows.
- Зафиксировать machine-checkable freshness checks для agent-facing docs:
  - ссылки на пути;
  - документированные команды и bin/package names;
  - наличие канонических agent docs и ожидаемой instruction layering.

## Impact
- Спецификация: `openspec/specs/dev-workflow/spec.md`
- Документация:
  - `AGENTS.md`
  - `README.md`
  - `docs/README.md`
  - `docs/guides/development-workflow.md`
  - `docs/BUILD_GUIDE.md`
  - `CONTRIBUTING.md`
  - новый curated раздел `docs/agent/**`
- Agent-facing артефакты:
  - локальные `AGENTS.md` в ключевых подпроектах
  - `.agents/skills/**`
  - doc/runbook freshness checks
- Tooling/validation:
  - существующие `scripts/check-*.py` и/или новые doc-smoke/doc-freshness проверки
  - CI/manual validation wiring для agent-facing drift

## Non-Goals
- Этот change НЕ меняет семантику анализатора, LSP, web-server или `bsl-agent`.
- Этот change НЕ требует широкого рефакторинга crate architecture.
- Этот change НЕ заменяет OpenSpec/Beads workflow; он делает его быстрее и понятнее для Codex.
- Этот change НЕ вводит line-level ссылки как основной стиль документации; link policy остаётся стабильной и path/section-first.
