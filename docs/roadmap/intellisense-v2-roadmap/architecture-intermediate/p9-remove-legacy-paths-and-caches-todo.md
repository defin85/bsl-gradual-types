# P9: TODO list — Удаление legacy путей и кэшей

**Дата:** 2026-01-10  
**Актуализировано:** 2026-01-12  
**Статус:** 🟢 Выполнено  
**Основание:** Фаза P9 из `docs/roadmap/intellisense-v2-roadmap/architecture-intermediate/salsa-migration-plan.md`

## Analysis

До завершения P9 в репозитории существовали два пути вычислений:

- **v2 (salsa / rust-analyzer style):** `AnalysisHostV2` + snapshots + writer thread + атомарный deps bundle.
- **legacy:** отдельный фасад системы типов + ad-hoc парсинг/IR в обработчиках + in-memory кэши + runtime переключатель.

Это приводило к проблемам:

- **Дублирование логики и расхождение поведения** (фикс делается в одном пути, регресс появляется в другом).
- **Риск mixed state/mixed deps** через “скрытые” legacy ветки и mutable state.
- **Сложность поддержки**: ветвления в рантайме усложняют дебаг и тестирование.

Цель P9: v2 становится **единственным** путём вычислений для LSP/CLI/Web; legacy путь и кэши удалены.

## Implementation Notes

- [x] LSP: удалён runtime переключатель legacy/v2, обработчики работают только через v2 snapshots/queries.
- [x] LSP endpoints: `definition` и прочие entrypoints переведены на v2.
- [x] Legacy caches: клиенты мигрированы на v2 queries; legacy кэши удалены.
- [x] Унификация: общие entrypoints intellisense используют `SemanticProgram` из `AnalysisV2::ir` и deps snapshot.
- [x] CLI: `AnalyzeIr` использует v2 host и query `ir` (v2-only).
- [x] Repo hygiene: убраны остаточные упоминания legacy идентификаторов/флагов по всему репозиторию (docs/tests/extension).

## DoD (P9 считается закрытым, если)

- [x] В LSP нет runtime ветвления legacy/v2; вычисления только v2.
- [x] В коде репозитория нет зависимостей от удалённых legacy фасада/кэшей.
- [x] CLI `AnalyzeIr` использует v2 host (`AnalysisHostV2` + `AnalysisV2::ir`) и не вызывает legacy IR build.
- [x] Repo-wide механические проверки не находят legacy идентификаторы/флаги.
- [x] Тесты/сборка проходят (см. Верификацию).

## Верификация (repo-wide)

### 1) Механические проверки (ожидается пусто)

```bash
rg -l "TypeSystem""Service" -S
rg -l "Analysis""Cache" -S
rg -l "Ir""Cache" -S
rg -l "BSL_INTELLISENSE_V2_""SALSA" -S
rg -l "use_""salsa_""v2" -S
```

### 2) CLI `AnalyzeIr` (v2-only)

```bash
rg -n "parse_and_analyze\\(" cli/src/main.rs -S
rg -n "ParserCoordinator::with_fallback\\(" cli/src/main.rs -S
```

### 3) Тесты

```bash
cargo test -p bsl-backend --bin bsl-lsp-server -- --color never
cargo test -p bsl-cli -- --color never
cargo test --workspace --no-run
```

### Факты (2026-01-12)

- Механические проверки из раздела (1) -> (пусто)
- `cargo test -p bsl-backend --bin bsl-lsp-server -- --color never`:
  ```text
  test result: ok. 20 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.31s
  ```
- `cargo test -p bsl-cli -- --color never`:
  ```text
  test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
  ```
- `cargo test --workspace --no-run`:
  ```text
  Finished `test` profile [unoptimized + debuginfo] target(s) in 49.50s
  ```
