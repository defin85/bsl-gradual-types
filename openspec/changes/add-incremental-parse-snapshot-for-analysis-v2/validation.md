# Validation

## Команды

```bash
cargo test -p bsl-analysis-v2 --lib
cargo test -p bsl-runtime --lib parser_coordinator -- --nocapture
cargo test -p bsl-backend --bin bsl-lsp-server --no-run
openspec validate add-incremental-parse-snapshot-for-analysis-v2 --strict --no-interactive
```

## Результат

- `bsl-analysis-v2`: PASS (`80 passed`)
- `bsl-runtime` (parser_coordinator): PASS (`5 passed`)
- `bsl-backend` (`bsl-lsp-server --no-run`): PASS
- `openspec validate ... --strict`: PASS

## Ограничения текущей итерации

- Пункт `3.2` (`range-limited recompute`) оставлен в статусе TODO.
