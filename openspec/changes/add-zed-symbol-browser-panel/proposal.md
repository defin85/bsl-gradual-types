# Change: Добавить панель Symbol Browser в Zed (через форк)

## Why
В VS Code у расширения BSL есть панель Type Repository с деревом типов, сгруппированным по метаданным. В Zed расширения не могут создавать UI-панели. Единственный путь — добавить панель прямо в ядро Zed через форк и предложить PR upstream.

Панель должна работать в двух режимах:
- **Generic**: для любого языка через `workspace/symbol` (группировка по SymbolKind)
- **BSL**: через кастомный LSP-метод `bsl/getAllTypes` (группировка по метаданным)

## What Changes
- Новый крейт `symbol_browser` в форке `defin85/zed`
- Панель с группировкой символов по kind (Classes, Functions, Modules...)
- Интеграция с LSP через `project.symbols()`
- Регистрация в workspace (actions, settings, panel dock)
- Openspec change в `bsl-gradual-types` для отслеживания

## Impact
- Новый capability: `zed-symbol-browser`
- Код в форке: `defin85/zed` (PR upstream при готовности)
- Не затрагивает: `bsl-gradual-types` ядро, VS Code расширение

## Non-Goals
- BSL-specific группировка (будет отдельным change)
- Интерактив (клик для перехода, сворачивание групп)
- Поиск/фильтр
