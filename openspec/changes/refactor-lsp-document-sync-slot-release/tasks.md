## 1. Specification
- [ ] 1.1 Добавить в `bsl-intellisense-v2` контракт short-lived transport path для `didOpen/didChange`, который освобождает transport slot после current-revision handoff и не ждёт slow stages inline.
- [ ] 1.2 Явно зафиксировать semantics `applied_version` после document-sync handoff и не смешивать её с readiness completion artifacts.
- [ ] 1.3 Уточнить churn-aware completion requirement так, чтобы second-scale pre-poll backlog от pending document-sync notifications считался регрессией.
- [ ] 1.4 Расширить representative real-module gate отдельным `didChange-burst` профилем, sample discipline и проверкой `service_future_to_first_poll_wait_ms` по численным budget'ам.

## 2. Design
- [ ] 2.1 Зафиксировать root-cause reasoning по incident bundle `2026-03-22T16:19:59Z`, `tower-lsp` concurrency slots и текущему `impl_document_sync` lifecycle.
- [ ] 2.2 Описать целевой handoff: `current revision apply -> background parse/head/exact/diagnostics -> immediate document-sync future completion`.
- [ ] 2.3 Явно перечислить, какие стадии остаются inline, а какие уходят в background worker/pipeline.
- [ ] 2.4 Описать инварианты для version tokens, supersession/cancellation и observability после переноса slow stages за transport boundary.
- [ ] 2.5 Зафиксировать отклонённые альтернативы: standalone concurrency uplift, ad-hoc `yield`, stale fallback.
- [ ] 2.6 Явно зафиксировать, что change не обещает сократить handler-internal `IR` / `type resolution` latency.

## 3. Validation
- [ ] 3.1 Провалидировать change: `openspec validate refactor-lsp-document-sync-slot-release --strict --no-interactive`.
- [ ] 3.2 Провести review change с владельцами LSP/document-sync и completion runtime, используя incident bundle как evidence.
