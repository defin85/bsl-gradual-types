# Change: Bounded stale completion fastpath под large-module churn

## Why
Даже с приоритетами и deferred diagnostics completion на больших модулях может ждать дорогой latest-path (parse/ir) и уходить в секундный хвост. Нужен явный fastpath, который гарантирует bounded latency через stale-compatible ответ при контролируемой свежести.

## What Changes
- **ADDED**: churn-aware completion fastpath с жестким latency budget для latest-path и немедленным stale fallback при допустимой свежести.
- **ADDED**: строгий stale-acceptance контракт (version gap, age, deps/settings compatibility) для completion under churn.
- **ADDED**: background refresh контракт после stale serve, чтобы система догоняла latest без блокировки пользователя.
- **ADDED**: quality gate профиль для large-module churn с pass/fail критериями по completion latency и stale usage.

## Impact
- Affected specs:
  - `bsl-intellisense-v2`
- Affected code (planned):
  - `bsl-runtime/src/application/intellisense_v2/facade.rs`
  - `bsl-runtime/src/application/intellisense_v2/policy.rs`
  - `backend/src/bin/lsp_server/server/core.rs`
  - `backend/src/bin/lsp_server/server/language_server.rs`

## Dependencies
- Depends on:
  - `add-incremental-parse-snapshot-for-analysis-v2`
  - `add-cancellable-diagnostics-supersession`

## Out of Scope
- Изменение логики ранжирования completion candidates.
- Разделение протокола LSP completion response на новый transport.
