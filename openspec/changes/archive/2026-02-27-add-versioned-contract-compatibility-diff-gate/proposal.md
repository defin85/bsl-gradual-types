# Change: Добавить compatibility-diff gate для versioned contracts

## Why
Текущий слой `contracts/**` уже фиксирует структуру и major-версии, но не умеет автоматически выявлять semantic breaking changes между версиями контрактов.

Из-за этого возможен сценарий, когда breaking изменение внутри того же `vN` проходит незамеченным: структура файлов валидна, но совместимость нарушена.

Нужен follow-up change, который добавит формальную compatibility-diff проверку (manual), чтобы:
- объективно различать non-breaking и breaking изменения контрактов;
- требовать major bump при breaking diff;
- требовать migration note при major bump;
- выдавать детерминированный отчёт pass/fail для review.

## What Changes
- **ADDED** requirement в `dev-workflow`: compatibility-diff gate сравнивает contract baseline и candidate contract по policy.
- **ADDED** requirement в `dev-workflow`: breaking diff без major bump должен падать.
- **ADDED** requirement в `dev-workflow`: при major bump обязателен migration note.
- **ADDED** requirement в `dev-workflow`: manual workflow/command должен публиковать machine-readable report.

## Impact
- Affected specs:
  - `dev-workflow`
- Affected code (implementation follow-up):
  - `scripts/` (новый compatibility-diff checker)
  - `.github/workflows/ci.yml` (manual `workflow_dispatch` job для compatibility-diff)
  - `contracts/**` (changelog discipline для major bumps)

## Dependencies
- Зависит от `add-versioned-contracts-layer` как от базового слоя контрактов и baseline структуры `contracts/**`.

## Non-Goals
- Делать compatibility-diff gate обязательным на каждый `push/pull_request`.
- Покрывать все возможные schema dialects и языки контрактов в первом шаге.
- Автоматически генерировать migration notes.
