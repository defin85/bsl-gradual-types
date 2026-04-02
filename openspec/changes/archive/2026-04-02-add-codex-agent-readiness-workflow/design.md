# Design: add-codex-agent-readiness-workflow

## Context
Аудит репозитория глазами нового Codex-агента показал повторяющийся паттерн:
- архитектура и рабочие entry points в проекте уже существуют и в целом понятны;
- но agent-facing “путь входа” раздроблен между `README.md`, `AGENTS.md`, `openspec/project.md`, `backend/README.md`, `scripts/README.md`, `bsl-agent/README.md`, CI и process docs;
- несколько первичных документов содержат устаревшие команды и пути, поэтому агент не может доверять первому найденному документу;
- `AGENTS.md` содержит много полезной process-информации, но в текущем виде перегружен и плохо работает как короткий dispatcher;
- Codex-specific bootstrap и повторяющиеся workflows не оформлены как отдельные, переиспользуемые артефакты.

Для long-running agent work это увеличивает “entropy budget”: рабочее состояние агента становится хрупким, потому что ему приходится каждый раз пересобирать карту проекта и локально верифицировать, какие команды вообще живы.

## Goals
- Дать новому Codex-агенту один канонический путь входа в репозиторий.
- Сократить root instruction surface и перенести детали ближе к рабочим директориям.
- Сделать run/test/verify path исполнимым, а не “предполагаемым”.
- Сделать Codex setup и recurring workflows portable и переиспользуемыми.
- Машинно ловить drift в agent-facing документации до merge.

## Non-Goals
- Не менять семантику анализатора, LSP, web-server или MCP.
- Не переписывать всю существующую документацию в одном change.
- Не заменять OpenSpec/Beads workflow новой process-моделью.
- Не делать line-level ссылки основным стилем навигации по документации.

## Decisions

### 1. Ввести curated agent-facing docs surface под `docs/agent/`
Нужен отдельный, небольшой и явно curated слой документации, ориентированный на нового агента, а не на исторические roadmap/doc dumps.

Минимальный набор:
- `docs/agent/index.md` — entry point и карта source-of-truth;
- `docs/agent/architecture-map.md` — workspace map, entry points, ключевые binary surfaces, важные crate boundaries;
- `docs/agent/verification.md` — run/test/verify contract с prerequisites и expected outcomes;
- `docs/agent/task-artifacts.md` — где искать OpenSpec, Beads, CI, readiness assets и execution evidence;
- `docs/agent/codex-setup.md` — portable Codex/MCP bootstrap и repo-local tooling notes.

Альтернатива “оставить всё в root README + AGENTS” отвергнута, потому что она снова превращает стартовую поверхность в длинный mixed-purpose документ.

### 2. Root `AGENTS.md` становится dispatcher, а детали уезжают в nested `AGENTS.md`
Root `AGENTS.md` должен отвечать только за:
- краткое назначение репозитория;
- глобальный workflow (OpenSpec -> Beads -> Code);
- ссылку на curated agent docs;
- указание, где искать локальные инструкции.

Локальные `AGENTS.md` нужны там, где у подпроекта есть свой entry point, verify path или toolchain:
- `backend/`
- `bsl-agent/`
- `vscode-extension/`

`AGENTS.override.md` использовать только в случае реального override, а не как ещё один способ “добавить локальные заметки”. Это сохраняет instruction chain предсказуемой.

### 3. Verification contract должен быть command-first и опираться на живые binary names
Runbook не должен ссылаться на исторические имена бинарников или “примерные” команды.

Канонический verification contract должен:
- использовать реальные текущие binary/package names (`bsl-cli`, `bsl-web-server`, `bsl-lsp-server`, `bsl-agent`);
- разделять проверки на smoke/manual/heavy;
- явно описывать prerequisites, expected outcomes и known non-default paths;
- быть связан с CI/manual gates и readiness checks.

Альтернатива “оставить verify path распределённым по README, scripts/README и CI YAML” отвергнута: это и создаёт текущий drift.

### 4. Portable Codex setup должен стать документом, а не зависеть от checked-in machine-specific config
Portable bootstrap нельзя строить вокруг checked-in конфигов с machine-specific абсолютными путями или чувствительными значениями.

Канонический Codex setup должен:
- использовать sanitized examples;
- объяснять, какие repo-local MCP/tooling integration действительно поддерживаются;
- отделять пример конфигурации от локального окружения конкретного разработчика.

Checked-in `.mcp.json` может оставаться вспомогательным артефактом, но не должен быть единственным или главным onboarding path для Codex.

### 5. Повторяющиеся workflows должны стать repo-local skills
Повторяемые, многосоставные и agent-specific workflow разумно оформить как `.agents/skills/**`, а не каждый раз описывать длинным prose в гайдах.

Минимальный стартовый набор:
- `verify-workspace`
- `bsl-agent-mcp-smoke`
- `openspec-delivery-matrix`
- `docs-drift-audit`

Это снижает стоимость повторных запусков и делает процесс ближе к progressive disclosure: короткий skill summary, затем `SKILL.md`, затем вспомогательные скрипты/референсы.

### 6. Drift в agent-facing документации нужно ловить машинно
Проверка “путь существует” уже полезна, но недостаточна.

Нужно дополнительно автоматизировать:
- сверку документированных package/bin names с реальным workspace;
- smoke-проверку ключевых документированных команд;
- наличие канонических agent docs и ожидаемой instruction layering;
- проверку, что root onboarding docs ссылаются на живые agent-facing entry points.

Важно, что эти проверки должны быть локально запускаемыми и подключаемыми к CI/manual gates.

### 7. Stable link policy: path/section-first, line links only for evidence
Для долговечных agent docs навигация должна быть устойчивой к дрейфу строк.

Поэтому:
- persistent docs используют ссылки на файл, раздел или символ по умолчанию;
- line-level ссылки допускаются только в review/audit/evidence docs или в generated references, где они проверяются/перестраиваются автоматически.

Это уменьшает стоимость сопровождения и не превращает docs refresh в бесконечную починку line anchors.

## Risks / Trade-offs
- Новый curated docs layer сам по себе может стать ещё одним источником спrawl.
  - Митигация: жёстко ограничить его ролью index/runbook/architecture map, а не переносить туда исторические roadmap-документы.
- Nested `AGENTS.md` могут начать дублировать root инструкции.
  - Митигация: root только dispatcher, локальные файлы только area-specific context.
- Repo-local skills тоже могут устареть.
  - Митигация: skills должны опираться на канонический runbook и попадать под freshness checks.

## Migration Plan
1. Сначала зафиксировать требования в `dev-workflow`.
2. Затем создать curated agent docs surface как новый source-of-truth слой.
3. После этого сократить root `AGENTS.md` и добавить nested `AGENTS.md`.
4. Затем выровнять первичные onboarding-доки под новый канонический runbook.
5. Потом добавить Codex setup docs, repo-local skills и doc freshness checks.
6. В конце подключить validation и убедиться, что drift ловится автоматически.
