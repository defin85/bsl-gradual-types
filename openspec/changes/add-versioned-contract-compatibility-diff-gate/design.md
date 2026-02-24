## Context

В `add-versioned-contracts-layer` введён baseline слой `contracts/**` и структурная проверка (`schema`/`version`/`migration note` для `major>1`).
Остаётся пробел: отсутствует semantic diff между candidate и baseline контрактами, поэтому breaking изменения внутри того же major не детектируются автоматически.

## Goals / Non-Goals

- Goals:
  - Добавить deterministic compatibility-diff policy для versioned contracts.
  - Ввести чёткую классификацию `non_breaking` vs `breaking`.
  - Зафиксировать manual gate с JSON report (`pass/fail`, причина, список нарушений).
- Non-Goals:
  - Делать full formal verification для всех видов JSON Schema.
  - Блокировать PR автоматически на этом этапе (manual-only rollout).

## Decisions

### Decision 1: Политика сравнения baseline → candidate формализуется на уровне contract payload

Compatibility-diff checker сравнивает:
- обязательные ключи и их типы;
- фиксированные enum/value-наборы для публичных label semantics;
- policy-поля (`breaking_change_requires_major_bump`, `breaking_change_requires_migration_note`).

Удаление/сужение публичных допустимых значений классифицируется как `breaking`.
Добавление новых необязательных значений (без удаления старых) классифицируется как `non_breaking`.

### Decision 2: Breaking без major bump запрещён

Если для surface обнаружен `breaking` diff между базовым контрактом и candidate, checker MUST падать, когда major версия не увеличена.

### Decision 3: Major bump без migration note запрещён

При major bump checker MUST проверять наличие migration note в `contracts/<surface>/vN/changelog.md`.

### Decision 4: Rollout manual-only

Compatibility-diff gate запускается отдельной ручной командой/`workflow_dispatch` job.
Это снижает риск ложных блокировок на раннем rollout и позволяет откалибровать policy.

## Risks / Trade-offs

- Риск ложных breaking срабатываний при слишком жёсткой классификации.
  - Митигация: начать с ограниченного policy-сета (labels/prefixes/enum semantics), расширять постепенно.
- Риск пропуска edge-case breaking изменений вне охваченной модели.
  - Митигация: явно документировать coverage compatibility-diff policy и добавлять regression fixtures.
- Риск ручного обхода gate (manual process).
  - Митигация: фиксировать отчёт в PR/change validation артефактах.

## Migration Plan

1. Ввести спецификацию compatibility-diff policy в `dev-workflow`.
2. Реализовать checker и формат отчёта.
3. Добавить manual workflow_dispatch gate.
4. Подготовить regression fixtures для известных breaking/non-breaking сценариев.

## Open Questions

- Нужна ли в следующем этапе автопубликация отчёта в PR комментарии (после стабилизации manual rollout).
