## 1. Specification
- [x] 1.1 Добавить delta в `bsl-intellisense` для server-driven startup orchestration индекса.
- [x] 1.2 Зафиксировать single-flight контракт full-index (startup/build без дублирования).
- [x] 1.3 Зафиксировать отказ от filesystem sentinel как источника истины при старте extension.

## 2. Backend (LSP) Contract
- [x] 2.1 Добавить/обновить custom request `bsl/getIndexState` (contract v1): `version`, `state`, `ready`, `active_operation`, `operation_id`, `message`, `updated_at_ms`.
- [x] 2.2 Реализовать single-flight guard для full-index (`startup` и `bsl/buildIndex`) без повторного запуска тяжёлого потока (повторный запрос attach к текущей операции).
- [x] 2.3 Добавить watchdog timeout (`hard_timeout_ms=1200000` default) для fail-safe перехода `running -> failed`.
- [x] 2.4 Добавить backend тесты на сценарии: startup-in-progress + повторный build request (attach); ready-state без повторного build; timeout переход в failed.

## 3. VS Code Extension Orchestration
- [x] 3.1 Перевести startup-решение о full build на `index state` от LSP.
- [x] 3.2 Убрать зависимость startup-решения от локального `unified_index.json` sentinel.
- [x] 3.3 Зафиксировать fail-closed startup policy для legacy LSP (если `getIndexState` не поддержан): без silent full build, с явным warning.
- [x] 3.4 Добавить UX-обработку ручного `Build Index` во время `running` как info-status `already running (attached)` без второй тяжёлой операции.
- [x] 3.5 Добавить extension tests на дедупликацию старта и корректное поведение при `running/ready/failed/idle` + legacy `Method not found`.

## 4. Validation
- [x] 4.1 Выполнить unit/integration тесты backend и extension для нового orchestration контракта.
- [x] 4.2 Выполнить `openspec validate update-startup-index-single-flight --strict --no-interactive`.
