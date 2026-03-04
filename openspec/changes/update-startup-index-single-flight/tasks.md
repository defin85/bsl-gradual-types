## 1. Specification
- [ ] 1.1 Добавить delta в `bsl-intellisense` для server-driven startup orchestration индекса.
- [ ] 1.2 Зафиксировать single-flight контракт full-index (startup/build без дублирования).
- [ ] 1.3 Зафиксировать отказ от filesystem sentinel как источника истины при старте extension.

## 2. Backend (LSP) Contract
- [ ] 2.1 Добавить/обновить custom request для чтения состояния индекса (`index state`) с machine-readable статусом.
- [ ] 2.2 Реализовать single-flight guard для full-index (`startup` и `bsl/buildIndex`) без повторного запуска тяжёлого потока.
- [ ] 2.3 Добавить backend тесты на сценарии: startup-in-progress + повторный build request; ready-state без повторного build.

## 3. VS Code Extension Orchestration
- [ ] 3.1 Перевести startup-решение о full build на `index state` от LSP.
- [ ] 3.2 Убрать зависимость startup-решения от локального `unified_index.json` sentinel.
- [ ] 3.3 Добавить extension tests на дедупликацию старта и корректное поведение при `running/ready/failed` состояниях.

## 4. Validation
- [ ] 4.1 Выполнить unit/integration тесты backend и extension для нового orchestration контракта.
- [ ] 4.2 Выполнить `openspec validate update-startup-index-single-flight --strict --no-interactive`.
