## ADDED Requirements

### Requirement: Репозиторий предоставляет curated agent-facing documentation surface для Codex (MUST)
Репозиторий MUST поддерживать отдельный канонический слой agent-facing документации под `docs/agent/`, предназначенный для нового Codex-агента.

Минимальный состав этого слоя MUST включать:
- `index.md` как единый стартовый индекс;
- `architecture-map.md` с картой workspace, entry points и ссылками на source-of-truth документы;
- `verification.md` с каноническим run/test/verify contract;
- `task-artifacts.md` с картой OpenSpec/Beads/CI/runtime артефактов и способом трассировки `Requirement -> Code -> Test`.

Этот слой MUST отвечать как минимум на вопросы:
- что это за проект;
- как устроен workspace;
- где основные entry points;
- как запускать, тестировать и верифицировать изменения.

#### Scenario: Новый Codex-агент находит стартовую карту из одного индекса
- **GIVEN** агент впервые входит в репозиторий без накопленного локального контекста
- **WHEN** он открывает `docs/agent/index.md`
- **THEN** он получает ссылки на каноническую карту архитектуры, verify runbook и карту task artifacts без необходимости начинать с исторических roadmap-документов

### Requirement: Инструкции `AGENTS.md` слоисты и пригодны для Codex-first onboarding (MUST)
Репозиторий MUST использовать layered instruction model для `AGENTS.md`.

Root `AGENTS.md` MUST:
- быть коротким dispatcher/index документом;
- описывать только глобальный workflow и корневые правила;
- ссылаться на канонический `docs/agent/index.md`;
- явно указывать, в каких директориях есть area-specific инструкции.

High-friction зоны с отдельным toolchain, entry points или verify path MUST иметь локальные `AGENTS.md`. Минимальный набор таких зон в рамках этого требования:
- `backend/`
- `bsl-agent/`
- `vscode-extension/`

`AGENTS.override.md` MUST использоваться только для intentional override родительских инструкций, а не как общий механизм добавления локальных заметок.

#### Scenario: Агент переходит в backend и получает локальные инструкции
- **GIVEN** новый Codex-агент начинает работу из директории `backend/`
- **WHEN** он определяет активную instruction chain
- **THEN** root `AGENTS.md` даёт короткий глобальный контекст
- **AND** `backend/AGENTS.md` даёт backend-specific entry points, verify commands и карту важных файлов без дублирования полного project playbook

### Requirement: Agent verification runbook является исполнимым и использует живые runtime surfaces (MUST)
Репозиторий MUST иметь канонический agent-facing verification runbook, который использует фактические текущие binary/package names и классифицирует проверки по стоимости.

Runbook MUST:
- документировать живые entry commands для `bsl-cli`, `bsl-web-server`, `bsl-lsp-server` и `bsl-agent`;
- разделять проверки минимум на `smoke` и `manual/heavy`;
- явно фиксировать prerequisites и expected outcomes;
- быть согласованным с актуальными CI/manual gates и readiness checks.

#### Scenario: Новый агент выполняет smoke path без археологии по README
- **GIVEN** чистый checkout репозитория и подготовленные минимальные prerequisites
- **WHEN** агент следует каноническому agent verification runbook
- **THEN** он находит один согласованный smoke path для запуска и проверки проекта
- **AND** этот путь использует текущие binary/package names, а не исторические или удалённые команды

### Requirement: Codex setup и recurring workflows оформлены как portable agent-facing артефакты (MUST)
Репозиторий MUST иметь канонический portable setup path для Codex и repo-local skills для повторяющихся agent workflow.

Portable setup path MUST:
- использовать sanitized examples без machine-specific абсолютных путей и без секретов;
- объяснять, как подключать поддерживаемые repo-local MCP/tooling integration;
- ссылаться на канонический agent-facing runbook, а не дублировать его.

Repo-local skills MUST существовать под `.agents/skills/` как минимум для следующих recurring workflows:
- workspace verification;
- `bsl-agent` MCP bootstrap/smoke;
- OpenSpec delivery matrix / `Requirement -> Code -> Test` evidence;
- audit drift в agent-facing документации.

#### Scenario: Агент поднимает Codex bootstrap без локально-зашитой конфигурации другого разработчика
- **GIVEN** новый Codex-агент работает в чистом локальном окружении
- **WHEN** он следует каноническому setup path и использует repo-local skills
- **THEN** ему не требуется зависеть от machine-specific checked-in конфигурации или секретов
- **AND** повторяющиеся workflow доступны как переиспользуемые skills

### Requirement: Drift в agent-facing документации и командах ловится машинно до merge (MUST)
Репозиторий MUST иметь machine-checkable validation для agent-facing documentation surface и документированных onboarding-команд.

Эта validation MUST как минимум ловить:
- ссылки на отсутствующие пути в agent-facing и первичных onboarding-документах;
- устаревшие package/bin names и broken documented commands;
- отсутствие канонических agent docs и ожидаемой instruction layering.

Validation MUST быть доступна как локальная команда и SHALL подключаться к CI/manual gate.

#### Scenario: Устаревшая команда в onboarding-доке не проходит validation
- **GIVEN** в `README.md`, `docs/README.md`, `docs/BUILD_GUIDE.md` или `docs/guides/development-workflow.md` появляется устаревшая команда или несуществующий binary name
- **WHEN** запускается agent-facing docs validation
- **THEN** проверка завершается fail до merge
- **AND** отчёт явно указывает, какой документ и какая команда больше не соответствуют фактическому workspace
