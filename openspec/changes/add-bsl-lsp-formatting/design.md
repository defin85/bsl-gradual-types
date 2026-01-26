# Design: LSP formatting для BSL

## Контекст
Target‑spec `bsl-intellisense-ide-grade` ожидает поддержку форматирования (SHOULD), но только при наличии согласованной стратегии форматтера.

Эта change реализует LSP formatting поверх решения из `openspec/changes/evaluate-bsl-formatting/design.md`.

## Принципы
- Детерминизм: одинаковый вход → одинаковые `TextEdit`.
- Минимальный diff: изменения по делу (без “переписывания” файла).
- Без блокирующего I/O в hot path (обработчик formatting).
- Явная конфигурация: форматирование можно отключить.

## Интеграция
- LSP: `textDocument/formatting` (и опционально `textDocument/rangeFormatting`).
- Поведение и ограничения (например, rangeFormatting на частично‑выбранном AST) должны быть описаны в spec и тестах.
