# Change: isolate auxiliary LSP CPU work from interactive transport path

## Why
Последнее расследование completion mixed-load latency показало, что UI/extension pre-send path не является основным виновником: `client_before_transport_write_wait_ms` остаётся малым, а пользовательская задержка собирается из server-side ingress/handoff backlog и auxiliary work на LSP runtime.

Кодовая проверка подтвердила два remediation-worthy источника starvation:
- background parse worker после `didOpen`/`didChange`/`didSave` выполняет `build_document_symbols(...)` уже на async LSP runtime, а не в bounded CPU path;
- `bsl.getCurrentContext` делает parse/context derivation inline внутри async handler.

Это конфликтует с уже существующим intent, что `documentSymbol` является auxiliary navigation surface и не должен задерживать interactive semantic responses. Активные change про backlog attribution закрывают observability blind spots, но не лечат сам runtime starvation.

## What Changes
- Уточнить contract в `bsl-intellisense-ide-grade`: auxiliary outline maintenance должна быть изолирована от interactive semantic path не только по outcome semantics, но и по runtime execution boundary.
- Добавить в `bsl-intellisense-v2` requirement, что CPU-heavy auxiliary LSP work (`documentSymbol` cache materialization / same-version refresh, `bsl.getCurrentContext` parse/context derivation) MUST выполняться через bounded blocking или эквивалентную isolated CPU boundary, а не inline на async transport/runtime loop.
- Добавить в `bsl-intellisense-v2` representative mixed-load regression guard, который budget-ит truthful seams `client_to_transport_wait_ms`, `service_future_to_first_poll_wait_ms` и `response_output_handoff_send_wait_ms`, а не только legacy `adapter_to_dispatch_wait_ms`.
- Зафиксировать remediation scope как server/runtime change, а не как UI/extension investigation.

## Impact
- Affected specs:
  - `bsl-intellisense-ide-grade`
  - `bsl-intellisense-v2`
- Affected code (implementation follow-up):
  - `backend/src/bin/lsp_server/server/language_server/impl_document_sync.rs`
  - `backend/src/bin/lsp_server/server/command_handlers.rs`
  - `backend/src/bin/lsp_server/server/core/tests.rs`
  - focused completion/documentSymbol/current-context runtime tests and perf artifacts

## Non-Goals
- Не перепроектировать целиком transport adapter, admission fairness или output queue architecture.
- Не менять semantic contract `documentSymbol` outcome taxonomy (`current_ready` / `latest_ready` / `unavailable`).
- Не решать generic completion query-body latency в этом change.
- Не переоткрывать UI/extension pre-send investigation без нового прямого контрдоказательства.
