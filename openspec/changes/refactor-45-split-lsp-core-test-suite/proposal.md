# Change: Декомпозиция монолитного LSP core test suite

## Why

В репозитории сейчас есть явный catastrophic outlier среди поддерживаемых test-only Rust модулей:

- `backend/src/bin/lsp_server/server/core/tests.rs` — `59_777 LOC`;
- следующий по размеру поддерживаемый test-only файл — `3_949 LOC`.

Текущий `core/tests.rs` уже не выглядит как один test module. По факту это несколько suite'ов,
слипшихся в один файл: synthetic regressions, diagnostics-save timeline coverage, completion /
current-context cases, live report probes и крупный общий harness layer.

Это создаёт практические риски:

- навигация и review становятся дорогими;
- любое изменение в LSP/backend acceptance tests повышает вероятность merge conflicts;
- безопасный mechanical refactor становится сложнее, чем сами тестовые правки;
- текущая large-file policy закрывает production `.rs` и inline tests, но не закрывает detached
  test-suite outliers такого масштаба.

## What Changes

- **ADDED (`dev-workflow`)**: repo-owned detached Rust test modules MUST NOT оставаться
  монолитными, если они превышают `10_000 LOC`.
- **ADDED (`dev-workflow`)**: refactor такого suite MUST раскладывать его в directory module
  (`tests/mod.rs` или эквивалент) с themed child modules и shared support harness.
- **ADDED (`dev-workflow`)**: decomposition MUST сохранять существующие test selectors / function
  names и текущие targeted validation commands, если отдельный approved change явно не меняет
  acceptance assets.
- **REFACTOR SCOPE**: разрезать `backend/src/bin/lsp_server/server/core/tests.rs` на
  `backend/src/bin/lsp_server/server/core/tests/mod.rs` плюс themed child modules и `support.rs`.
- **REFACTOR SCOPE**: вынести heavy live/report probes (`p45..p56` family или семантически
  эквивалентные группы) из того же файла, где живут synthetic/core regressions.

## Impact

- Affected specs:
  - `dev-workflow`
- Affected code:
  - `backend/src/bin/lsp_server/server/core.rs`
  - `backend/src/bin/lsp_server/server/core/tests.rs`
  - `backend/src/bin/lsp_server/server/core/tests/**`
- Affected validation:
  - таргетные `cargo test -p bsl-backend --bin bsl-lsp-server <selector> -- --nocapture`
  - `cargo test -p bsl-backend --bin bsl-lsp-server --no-run`

## Non-Goals

- Изменение runtime/LSP поведения production кода.
- Изменение смысловой нагрузки test coverage или переписывание acceptance semantics.
- Репозиторный policy на все test-only файлы `>3000 LOC`; этот change сознательно узок и
  закрывает catastrophic outlier class и конкретный `core/tests.rs`.
