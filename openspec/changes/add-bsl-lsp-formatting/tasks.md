## 1. Prereq: formatting strategy
- [ ] Использовать выбранную стратегию из `openspec/changes/evaluate-bsl-formatting/design.md`.
- [ ] Зафиксировать ограничения/границы: что форматируем (BSL), что не форматируем (например, внешние DSL).

## 2. LSP: Formatting
- [ ] Объявить поддержку форматирования в `ServerCapabilities`:
  - [ ] `document_formatting_provider`
  - [ ] (опционально) `document_range_formatting_provider`
- [ ] Реализовать `textDocument/formatting`:
  - [ ] Детерминированный результат (одинаковый ввод → одинаковый вывод).
  - [ ] Минимальный diff (не “переписывать” весь файл без необходимости).
  - [ ] Без блокирующего I/O в обработчике (форматтер и данные должны быть в памяти).
- [ ] Реализовать `textDocument/rangeFormatting` (если выбрано) с понятным поведением на границах диапазона.
- [ ] Конфигурация/настройки: явный флаг включения форматирования.

## 3. Tests
- [ ] Интеграционные тесты LSP:
  - [ ] `textDocument/formatting` возвращает ожидаемые правки на фикстурах.
  - [ ] Повторный запрос на одинаковом тексте возвращает идентичные правки.

## 4. Spec
- [ ] Обновить `openspec/changes/add-bsl-lsp-formatting/specs/bsl-intellisense/spec.md`.

## 5. Validation
- [ ] `openspec validate add-bsl-lsp-formatting --strict --no-interactive`
