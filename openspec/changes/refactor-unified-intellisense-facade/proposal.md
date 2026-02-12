# Change: refactor-unified-intellisense-facade

## Why
Сейчас orchestration одного и того же v2 анализа расползся между тремя адаптерами (`LSP`, `bsl-agent`, `web`):
- в LSP есть отдельный runtime и централизованные perf/observability решения,
- в `bsl-agent` и web остаются дублирующиеся ad-hoc цепочки `AnalysisHostV2`.

Из-за этого исправления производительности и отмены запросов нужно переносить вручную в несколько мест, что приводит к дрейфу поведения и регрессиям хвостовых задержек.

Нужно перейти на единую архитектуру без MVP-среза: полная миграция всех semantic-path интерфейсов на общий фасад в одном change.

## What Changes
- Ввести единый shared orchestration фасад IntelliSense v2 в `bsl-runtime`.
- Вынести/унифицировать runtime-семантику (wait/snapshot/deps update/cancellation/queueing) в общий слой, используемый всеми адаптерами.
- Перевести LSP, web и `bsl-agent` на общий фасад для semantic операций (completion/hover/signatureHelp/definition/diagnostics и related operations).
- Централизовать performance-политику (lazy `parse_result`, cancellation policy, bounded blocking) и сделать её общей для всех интерфейсов.
- Удалить дубли orchestration из адаптеров; оставить в них только transport mapping (LSP/HTTP/MCP DTO).
- Добавить cross-interface parity и perf regression тесты, чтобы предотвратить повторный дрейф.

## Impact
- Affected specs:
  - `bsl-intellisense-v2`
  - `mcp-bsl-agent`
- Affected code:
  - `bsl-runtime/src/application/*`, `bsl-runtime/src/system/*`
  - `backend/src/bin/lsp_server/server/*`
  - `backend/src/presentation/web/handlers.rs`
  - `bsl-agent/src/session/mod.rs`
- External API impact:
  - LSP/MCP/HTTP публичные контракты сохраняются
  - изменения касаются внутренней архитектуры, перф-политик и observability parity
