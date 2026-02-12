## 1. Specification
- [ ] 1.1 Добавить spec delta в `bsl-intellisense` для объединения sidebar panels в один activity bar контейнер.
- [ ] 1.2 Зафиксировать требования на консистентность счётчиков/метрик между `Overview`, `Type Repository`, `Diagnostics`, `Quick Actions`.
- [ ] 1.3 Зафиксировать запрет на хардкодные счётчики/сырые UI tokens в user-facing sidebar UI.

## 2. Design
- [ ] 2.1 Описать единый snapshot/state contract для sidebar данных (источник истины + refresh policy).
- [ ] 2.2 Описать миграцию view container: `bslAnalyzerCache` -> `bslAnalyzer` с обратной совместимостью команд.
- [ ] 2.3 Описать policy обработки `n/a`/missing метрик без противоречий и misleading статусов.

## 3. Implementation (follow-up)
- [ ] 3.1 Обновить `vscode-extension/package.json`: один activity bar контейнер и единая структура views.
- [ ] 3.2 Обновить регистрацию providers/refresh в `vscode-extension/src/extension.ts` под unified sidebar.
- [ ] 3.3 Убрать статические значения в Quick Actions (`frontend/src/vscode/quick_actions_panel.rs`) и подставлять live stats.
- [ ] 3.4 Синхронизировать providers (`overview`, `diagnostics`, `type repository`, `cache`) по единому snapshot contract.
- [ ] 3.5 Добавить regression-тесты на консистентность счётчиков и отсутствие сырых UI tokens.

## 4. Validation
- [ ] 4.1 `openspec validate update-vscode-sidebar-unified-panel-consistency --strict --no-interactive`
- [ ] 4.2 Review change с владельцами VS Code extension и LSP custom requests.
