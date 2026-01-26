# Design: LSP symbols for BSL (documentSymbol / workspaceSymbol)

## Цели
- Дать IDE структуру документа (Outline) и быстрый поиск по символам (Go to Symbol in Workspace).

## Источники правды
- Парсинг/IR через v2 pipeline (как в completion/hover/signatureHelp).

## Модель символов (минимум)
`textDocument/documentSymbol`:
- Верхний уровень: `Процедура`/`Функция` (SymbolKind: Function/Method).
- Вложенность: по возможности группировать по `#Область` (SymbolKind: Namespace) и/или по модулю.

`workspace/symbol`:
- Поиск по имени: возвращать `SymbolInformation`/`WorkspaceSymbol` с `Location`.
- MVP‑ограничение: покрытие только тех документов, которые доступны из текущего состояния сервера (например, open files / runtime index snapshot). Ограничение должно быть явно отражено в документации/спеке.
- MVP‑поведение:
  - поиск по подстроке без учёта регистра,
  - покрытие: только открытые документы,
  - лимит выдачи: 200 элементов.

## Риски
- Позиции: соответствие UTF‑16 позициям LSP и byte offsets в анализе (важно для корректных ranges).
- Инкрементальность: символы должны обновляться вместе с текстом документа и не “миксоваться” между ревизиями.
