# Change: bound `bsl.getCurrentContext` parse fan-out before blocking CPU admission

## Why
Incident bundle `2026-04-09T19-25-20Z` показал, что `bsl.getCurrentContext` остаётся крупным источником background CPU contention даже после того, как completion ingress/output seams уже здоровы:

- `intellisense_v2_current_context_parse_source_total_source_parser_coordinator = 8`, `ready_snapshot = 1`;
- `intellisense_v2_current_context_parse_ms_source_parser_coordinator p50 = 21217ms`, `p95 = 29385ms`;
- `intellisense_v2_current_context_wall_ms_source_parser_coordinator p50 = 23367ms`, `p95 = 31756ms`.

Read-only разбор по коду подтвердил архитектурную причину:

- каждый `bsl.getCurrentContext` сейчас сначала входит в `spawn_bounded_blocking(...)` в [command_handlers.rs](/home/egor/code/bsl-gradual-types/backend/src/bin/lsp_server/server/command_handlers.rs);
- duplicate suppression происходит только глубже, внутри sync `ParserCoordinator` через `Condvar`, уже после получения blocking CPU permit;
- follower-запросы therefore не делают второй parse, но продолжают занимать scarce blocking capacity и wall time, пока лидер парсит тот же текст;
- при cursor burst это создаёт лишний background pressure для didSave follow-up и других auxiliary paths без пользы для client-visible latest-only semantics.

## What Changes
- Зафиксировать в `bsl-intellisense-v2`, что same-file same-revision `bsl.getCurrentContext` parse/context derivation MUST broker-иться до входа в blocking CPU boundary, а не после него.
- Потребовать server-owned latest-only parse broker для `bsl.getCurrentContext`, который:
  - предпочитает exact ready parse snapshot текущей revision;
  - допускает не более одного leader parse/context derivation на эквивалентный `(file, revision/text)` key;
  - держит follower-requests в async/shared wait surface или завершает их bounded empty outcome при supersession/budget exhaustion;
  - не позволяет follower-запросам получать независимый blocking CPU permit только ради ожидания leader parse.
- Зафиксировать, что newest-generation-wins semantics остаётся fail-closed: stale response нельзя показывать как current context для newer generation, даже если лидерский parse всё ещё прогревает reusable artifact.
- Добавить dedicated observability/acceptance для broker role/outcome, чтобы operator мог отличать `ready_snapshot`, `broker_leader`, `broker_follower`, `superseded`, `budget_exhausted`.

## Implementation Order
Это первый change в серии. Он должен быть реализован до runtime/apply hardening и до completion collect optimization, чтобы убрать самый грубый auxiliary CPU storm и получить чище post-fix evidence для следующих этапов.

## Impact
- Affected specs:
  - `bsl-intellisense-v2`
- Affected code (implementation follow-up):
  - `backend/src/bin/lsp_server/server/command_handlers.rs`
  - `backend/src/bin/lsp_server/server/core/tests.rs`
  - `bsl-runtime/src/system/parser_coordinator.rs`
  - `bsl-runtime/src/system/basic_observability.rs`
  - `bsl-runtime/src/system/basic_observability/query_metrics.rs`
  - `bsl-runtime/src/system/system_coordinator/coordinator/observability.rs`
  - representative incident/live evidence under `backend/tests/perf/reports/`

## Non-Goals
- Не менять cadence/частоту client-side `bsl.getCurrentContext` запросов в VS Code.
- Не возвращать stale current-context response вместо newest-generation response.
- Не перепроектировать весь `ParserCoordinator` для всех его call sites.
- Не лечить writer/apply backlog этим change.

## Resolved Assumptions
- Owner parse broker остаётся на server/backend стороне рядом с `handle_get_current_context`, потому что именно там доступны client generation hints, ready snapshots и request-scoped supersession facts.
- Leader parse MAY продолжать прогрев reusable artifact после того, как отдельный follower запрос уже завершён bounded empty outcome; это не нарушает latest-only contract, если stale response не уходит клиенту как newest current context.
