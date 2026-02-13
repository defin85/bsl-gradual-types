# Change: Единая панель VS Code sidebar + консистентные метрики/счётчики BSL Analyzer

## Why
Текущий UX расширения показывает рассинхроны и дублирование в sidebar:
- две отдельные activity bar панели (`BSL Analyzer` и `BSL Cache`) вместо единого входа;
- противоречивые значения между `Overview`, `Diagnostics`, `Type Repository` и `Quick Actions`;
- отображение неотрендеренных UI токенов и устаревших/статических значений (например, фиксированный счётчик типов).

Это подрывает доверие к данным расширения и усложняет поддержку, потому что пользователь не понимает, какой виджет является источником истины.

## What Changes
- Объединить две activity bar панели расширения в одну (`BSL Analyzer`) и перенести cache dashboard в тот же контейнер.
- Зафиксировать единый контракт источника данных для sidebar-виджетов:
  - `Overview`, `Type Repository`, `Diagnostics`, `Quick Actions`, `Cache Dashboard` должны опираться на согласованные snapshots.
- Зафиксировать консистентность счётчиков:
  - значения `TypeRepository`/`Platform`/`Configuration` не должны противоречить друг другу между виджетами.
- Убрать статические/хардкодные UI-подписи в Quick Actions (например, фиксированный `3927 типов`) в пользу live-данных.
- Зафиксировать UX-контракт рендера:
  - пользователь не должен видеть сырые internal tokens вида `$(check)` и аналогичные неразобранные маркеры.

## Impact
- Affected specs:
  - `bsl-intellisense`
- Affected code (follow-up):
  - `vscode-extension/package.json` (viewsContainers/views/menus)
  - `vscode-extension/src/extension.ts` (регистрация sidebar providers/refresh commands)
  - `vscode-extension/src/providers/overviewProvider.ts`
  - `vscode-extension/src/providers/cacheDashboardProvider.ts`
  - `vscode-extension/src/providers/diagnosticsProvider.ts`
  - `vscode-extension/src/providers/hierarchicalTypeProvider.ts`
  - `vscode-extension/src/providers/actionsWebview.ts`
  - `frontend/src/vscode/quick_actions_panel.rs`

## Non-Goals
- Полный редизайн визуального стиля webview/деревьев.
- Изменение LSP протокола или перенос метрик за пределы существующих extension/LSP контрактов.
- Рефакторинг не связанных с sidebar подсистем расширения.
