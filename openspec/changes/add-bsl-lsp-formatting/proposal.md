# Change: add-bsl-lsp-formatting

## Why
Format Document / Format Selection — типичная IDE‑grade возможность. В текущем LSP контракте форматирование не заявлено и не реализовано, поэтому пользователю приходится полагаться на внешние инструменты/ручной стиль.

В проекте уже есть change `evaluate-bsl-formatting`, который выбирает стратегию форматирования. Этот change добавляет **реализацию** форматирования в LSP поверх выбранной стратегии и делает поведение в IDE явным и тестируемым.

## What Changes
- Добавить поддержку `textDocument/formatting` (и при необходимости `textDocument/rangeFormatting`) в LSP‑сервере.
- Добавить конфигурацию: форматирование можно включить/выключить (по умолчанию — в соответствии с выбранной стратегией).
- Зафиксировать контракт форматирования в `openspec/specs/bsl-intellisense/spec.md` (delta).
- Добавить тесты (интеграционные/регрессионные) на детерминизм и “минимальный diff”.

## Impact
- Спецификация: `openspec/specs/bsl-intellisense/spec.md` (delta).
- Код: `backend/src/bin/lsp_server/` (capabilities + handlers) и выбранный слой форматтера.
- Тесты: интеграционные тесты LSP на formatting/rangeFormatting.
