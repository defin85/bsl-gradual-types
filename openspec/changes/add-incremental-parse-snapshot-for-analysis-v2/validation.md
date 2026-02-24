# Validation

## Команды

```bash
cargo test -p bsl-analysis-v2 --lib
cargo test -p bsl-runtime --lib parser_coordinator -- --nocapture
cargo test -p bsl-backend --bin bsl-lsp-server --no-run
openspec validate add-incremental-parse-snapshot-for-analysis-v2 --strict --no-interactive
```

## Результат

- `bsl-analysis-v2`: PASS (`82 passed`)
- `bsl-runtime` (parser_coordinator): PASS (`5 passed`)
- `bsl-backend` (`bsl-lsp-server --no-run`): PASS
- `openspec validate ... --strict`: PASS

## Покрытие 3.2 (range-limited recompute)

- Добавлены таргетные тесты на безопасный range-aware reuse:
  - `ir_reuses_previous_version_for_tail_whitespace_append_snapshot`
  - `ir_does_not_reuse_previous_version_for_non_tail_snapshot_change`
