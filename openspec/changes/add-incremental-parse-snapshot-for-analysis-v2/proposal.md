# Change: Инкрементальный parse snapshot для analysis-v2

## Why
На больших модулях under churn completion-path получает длинный хвост из-за дорогого синтаксического пересчета на каждой ревизии. По текущему gate-репорту `large/warm` видно доминирование parse/query стадий (`syntax_diagnostics_query` и `ir_query_completion` в десятках секунд), при этом queue wait не является главным bottleneck.

## What Changes
- **ADDED**: version-bound `ParseSnapshot` контракт в v2 pipeline как единый источник parse state для интерактивных и diagnostics операций.
- **ADDED**: инкрементальное обновление parse snapshot по `didChange` через `tree_sitter` old-tree/edit path с фиксированным full-parse fallback при несогласованности.
- **ADDED**: changed-ranges aware invalidation для downstream стадий (syntax/IR), чтобы тяжелые пересчеты ограничивались затронутыми диапазонами.
- **ADDED**: observability сигналы для parse reuse, fallback причин и размера changed ranges.

## Impact
- Affected specs:
  - `bsl-intellisense-v2`
- Affected code (planned):
  - `analysis-v2/src/lib.rs`
  - `syntax/src/lib.rs`
  - `bsl-runtime/src/system/parser_coordinator.rs`
  - `backend/src/bin/lsp_server/server/language_server.rs`
  - `backend/src/bin/lsp_server/server/core.rs`

## Dependencies
- Foundation change for:
  - `add-cancellable-diagnostics-supersession`
  - `add-bounded-stale-completion-fastpath`

## Out of Scope
- Изменение пользовательской семантики diagnostics/completion candidates.
- Полная замена существующего salsa query graph на иной вычислительный движок.
