# Change: Добавить панель Symbol Browser в Zed (через форк)

## Why
В VS Code у расширения BSL есть панель Type Repository с деревом типов, сгруппированным по метаданным. В Zed расширения не могут создавать UI-панели. Единственный путь — добавить панель прямо в ядро Zed через форк и предложить PR upstream.

В этом change панель работает в **Generic** режиме: для любого языка через `workspace/symbol` с группировкой по `SymbolKind`.

BSL-specific режим через кастомный LSP-метод `bsl/getAllTypes` и группировку по метаданным вынесен из текущего scope в отдельную follow-up задачу `bsl-gradual-types-zkxt`.

## What Changes
- Новый крейт `symbol_browser` в форке `defin85/zed`
- Панель с группировкой символов по kind (Classes, Functions, Modules...)
- Интеграция с LSP через `project.symbols()`
- Регистрация в workspace (actions, settings, panel dock)
- BSL LSP contract fix: пустой `workspace/symbol` query возвращает доступные open-document symbols для generic-панели
- Openspec change в `bsl-gradual-types` для отслеживания

## Impact
- Новый capability: `zed-symbol-browser`
- Код в форке: `defin85/zed` (PR upstream при готовности)
- Код в `bsl-gradual-types`: BSL LSP `workspace/symbol` empty-query contract
- Не затрагивает: VS Code расширение

## Non-Goals
- BSL-specific `bsl/getAllTypes` режим и группировка по метаданным (отдельная follow-up задача `bsl-gradual-types-zkxt`)
- Интерактив (клик для перехода, сворачивание групп)
- Поиск/фильтр
