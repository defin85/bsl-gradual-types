## 1. Contract
- [x] 1.1 Добавить в `dev-workflow` requirement для curated agent-facing documentation surface под `docs/agent/`.
- [x] 1.2 Добавить в `dev-workflow` requirement для layered `AGENTS.md` contract:
  - [x] root `AGENTS.md` как короткий dispatcher/index
  - [x] локальные `AGENTS.md` в `backend/`, `bsl-agent/`, `vscode-extension/`
  - [x] `AGENTS.override.md` только как исключение для intentional override
- [x] 1.3 Добавить в `dev-workflow` requirement для executable agent verification runbook:
  - [x] живые команды для основных binary surfaces
  - [x] разделение smoke/manual/heavy
  - [x] prerequisites и expected outcomes
- [x] 1.4 Добавить в `dev-workflow` requirement для portable Codex setup и repo-local skills.
- [x] 1.5 Добавить в `dev-workflow` requirement для machine-checkable freshness checks agent-facing docs и команд.

## 2. Curated Agent Docs
- [x] 2.1 Создать `docs/agent/index.md` как единый стартовый индекс для нового Codex-агента.
- [x] 2.2 Создать `docs/agent/architecture-map.md` с картой workspace, entry points и source-of-truth ссылками.
- [x] 2.3 Создать `docs/agent/verification.md` с каноническими командами run/test/verify и expected outcomes.
- [x] 2.4 Создать `docs/agent/task-artifacts.md` с картой OpenSpec/Beads/CI/runtime артефактов и способом трассировки `Requirement -> Code -> Test`.
- [x] 2.5 Зафиксировать authoring policy для agent docs:
  - [x] durable docs используют path/section links по умолчанию
  - [x] line-level links допустимы только для review/evidence/generated references

## 3. Instruction Layering
- [x] 3.1 Переписать root `AGENTS.md` в короткий dispatcher, который ссылается на curated agent docs и не дублирует длинные playbook sections.
- [x] 3.2 Добавить `backend/AGENTS.md` с backend/LSP/web entry points, локальными verify командами и картой важных файлов.
- [x] 3.3 Добавить `bsl-agent/AGENTS.md` с MCP-specific runbook, smoke path и runtime artifacts.
- [x] 3.4 Добавить `vscode-extension/AGENTS.md` с Node/tooling/test workflow и ссылками на relevant docs.

## 4. Onboarding Docs Alignment
- [x] 4.1 Обновить `README.md`, чтобы он ссылался на живые binary/package names и актуальный CI/verify contract.
- [x] 4.2 Обновить `docs/README.md`, чтобы он ссылался на существующие разделы и agent-facing index.
- [x] 4.3 Обновить `docs/BUILD_GUIDE.md` и `docs/guides/development-workflow.md` под текущие entry points и verify flow.
- [x] 4.4 Обновить `CONTRIBUTING.md`, чтобы требования к локальным проверкам и workflow не расходились с каноническим runbook.

## 5. Codex Setup And Skills
- [x] 5.1 Создать `docs/agent/codex-setup.md` с portable Codex/MCP bootstrap и sanitized config examples.
- [x] 5.2 Добавить repo-local skill `verify-workspace` для короткого smoke/verify workflow.
- [x] 5.3 Добавить repo-local skill `bsl-agent-mcp-smoke` для MCP bootstrap и smoke-проверки.
- [x] 5.4 Добавить repo-local skill `openspec-delivery-matrix` для сборки `Requirement -> Code -> Test` evidence.
- [x] 5.5 Добавить repo-local skill `docs-drift-audit` для agent-facing документации и runbook drift.

## 6. Freshness Checks
- [x] 6.1 Расширить path/link checks на `docs/agent/**` и agent-facing onboarding docs.
- [x] 6.2 Добавить command-smoke/doc-freshness check на устаревшие package/bin names и broken documented commands.
- [x] 6.3 Добавить проверку на наличие канонических agent docs и ожидаемой instruction layering.
- [x] 6.4 Подключить эти проверки к локальной validation-команде и CI/manual gate.

## 7. Validation
- [x] 7.1 Прогнать `openspec validate add-codex-agent-readiness-workflow --strict --no-interactive`.
- [x] 7.2 Подтвердить traceability `Requirement -> Artifact -> Validation` для каждого нового requirement.
