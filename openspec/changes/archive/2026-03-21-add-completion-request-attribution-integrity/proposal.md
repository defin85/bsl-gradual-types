# Change: add-completion-request-attribution-integrity

## Почему
Текущий `Completion Timeline v7` уже умеет сужать pre-method lag до `transport_received -> service_scope_entered -> method_entered`, но при overlap completion в одной позиции остаётся best-effort handoff через pending registry, привязанный к `uri + line + character`.

На реальных incident bundle это уже даёт подозрительные traces с одинаковым `transport_received_at_ms` у разных `request_id`. В такой ситуации derived handoff может переоценивать `server_before_method_entry`, хотя оператору нужен либо request-bound факт, либо явное признание, что pre-method attribution сейчас недоказан.

## Что меняется
- Поднимаем authoritative `bsl.getCompletionTimeline` contract до `v8` и добавляем bounded provenance для pre-method attribution.
- Фиксируем, что request-bound pre-method facts считаются сильными только при подтверждённой integrity; best-effort fallback должен быть явно помечен и не должен маскироваться под authoritative same-request attribution.
- Обновляем Completion Timeline panel, clipboard и incident bundle summary, чтобы они показывали provenance и не агрегировали weak attribution как сильный ingress bottleneck.
- Добавляем regression coverage и smoke/runbook expectations для overlapping completion requests в одной позиции.

## Non-Goals
- Этот change не чинит сам root cause больших задержек до `service_scope_entered`.
- Этот change не вводит новый лог-канал и не переводит observability на free-text события.
- Этот change не меняет completion semantics, только integrity и читаемость root-cause attribution.

## Impact
- Affected specs:
  - `bsl-intellisense-v2`
  - `bsl-intellisense`
- Affected code:
  - `backend/src/bin/lsp_server/server/request_context.rs`
  - `backend/src/bin/lsp_server/server/language_server/impl_completion.rs`
  - `backend/src/bin/lsp_server/types.rs`
  - `vscode-extension/src/lsp/customRequests.ts`
  - `vscode-extension/src/providers/completionTimeline*.ts`
  - `vscode-extension/src/providers/observabilityIncidentBundle*.ts`
