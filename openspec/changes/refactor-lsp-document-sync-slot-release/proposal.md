# Change: short-lived transport path для LSP document sync under completion churn

## Why
Incident bundle `2026-03-22T16:19:59Z` показывает, что текущий tail у completion возникает не внутри handler, а раньше: requests `73`, `77` и `96` ждут первый `poll()` service future `5857ms`, `14754ms` и `6040ms`, хотя dispatcher для них уже `ready`.

Кодовая картина совпадает с этим поведением:
- `backend/src/bin/lsp_server/main.rs` запускает `tower-lsp` без явной настройки `concurrency_level`;
- `tower-lsp` transport обрабатывает request futures через ограниченный `buffer_unordered`;
- `backend/src/bin/lsp_server/server/language_server/impl_document_sync.rs` публикует current revision рано, но затем продолжает держать `didOpen/didChange` service future живой, пока inline ждёт slow stages вроде `parse snapshot build`.

Из-за этого long-lived document-sync notifications могут занимать transport slots и держать последующие completion futures в состоянии "created, but not first-polled". Текущая спецификация уже требует bounded fail-closed completion, но пока не фиксирует архитектурный контракт "document-sync future быстро освобождает transport slot и уводит slow work в background orchestration".

## What Changes
- Добавить в `bsl-intellisense-v2` требование short-lived transport path для `didOpen/didChange`:
  - current-revision `SetFile` handoff и semantics `applied_version` фиксируются явно;
  - slow stages (`parse snapshot`, current-revision precompute, exact precompute, deferred diagnostics) продолжаются в фоне;
  - transport slot не удерживается ожиданием этих slow stages.
- Уточнить churn-aware completion contract: интерактивный completion не должен накапливать секундный `service_future_created -> first poll` backlog только потому, что предыдущие document-sync notifications ещё не завершили slow background work.
- Усилить representative real-module gate:
  - прогон через реальный LSP transport path, а не только прямой service harness;
  - отдельный `didChange-burst` профиль;
  - отдельная проверка `service_future_to_first_poll_wait_ms` с численным pass/fail budget, чтобы регрессия slot-retention не маскировалась поздним успешным exact upgrade.

## Impact
- Affected specs:
  - `bsl-intellisense-v2`
- Affected code (implementation follow-up):
  - `backend/src/bin/lsp_server/main.rs`
  - `backend/src/bin/lsp_server/server/language_server/impl_document_sync.rs`
  - `backend/src/bin/lsp_server/server/core/deps_and_precompute.rs`
  - `backend/src/bin/lsp_server/server/core/tests.rs`
  - acceptance / observability regression harness вокруг completion timeline

## Non-Goals
- Простое увеличение `tower-lsp` concurrency как substitute для root-cause fix.
- Перепроектирование `CompletionHeadArtifact` или full IR publish model в этом change.
- Новый fallback на stale semantic payload, keyword completion или guessed current-revision substitute.
- Новый observability contract beyond existing completion timeline fields; change использует текущие поля как acceptance evidence.
- Сокращение handler-internal latency для `IR` / `type resolution` / `exact artifact readiness`, если completion уже начал исполняться.
