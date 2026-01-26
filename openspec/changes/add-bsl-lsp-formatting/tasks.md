## 1. Prereq: formatting strategy
- [x] Использовать выбранную стратегию из `openspec/changes/evaluate-bsl-formatting/design.md`.
- [x] Зафиксировать ограничения/границы: что форматируем (BSL), что не форматируем (например, внешние DSL).

## 2. LSP: Formatting
- [x] Объявить поддержку форматирования в `ServerCapabilities`:
  - [x] `document_formatting_provider`
  - [ ] (опционально) `document_range_formatting_provider`
- [x] Реализовать `textDocument/formatting`:
  - [x] Детерминированный результат (одинаковый ввод → одинаковый вывод).
  - [x] Минимальный diff (не “переписывать” весь файл без необходимости).
  - [x] Без блокирующего I/O в обработчике (форматтер и данные должны быть в памяти).
- [ ] Реализовать `textDocument/rangeFormatting` (если выбрано) с понятным поведением на границах диапазона.
- [x] Конфигурация/настройки: явный флаг включения форматирования.

## 3. Tests
- [x] Интеграционные тесты LSP:
  - [x] `textDocument/formatting` возвращает ожидаемые правки на фикстурах.
  - [x] Повторный запрос на одинаковом тексте возвращает идентичные правки.

## 4. Spec
- [x] Обновить `openspec/changes/add-bsl-lsp-formatting/specs/bsl-intellisense/spec.md`.

## 5. Validation
- [x] `openspec validate add-bsl-lsp-formatting --strict --no-interactive`
