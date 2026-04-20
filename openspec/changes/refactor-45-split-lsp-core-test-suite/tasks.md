## 1. Baseline And Contract

- [x] 1.1 Зафиксировать baseline inventory для catastrophic detached test-suite outlier и
      resulting split plan для `backend/src/bin/lsp_server/server/core/tests.rs`.
- [x] 1.2 Зафиксировать parity contract: production behavior не меняется, существующие test
      selectors сохраняются, targeted validation commands продолжают работать.

## 2. Test-Suite Decomposition

- [x] 2.1 Заменить плоский `backend/src/bin/lsp_server/server/core/tests.rs` на directory module
      `backend/src/bin/lsp_server/server/core/tests/mod.rs` с общим `support.rs`.
- [x] 2.2 Разложить synthetic/core regressions и diagnostics-save families по themed child modules
      с reviewable размером файла.
- [x] 2.3 Вынести heavy live/report probe family (`p45..p56` или семантически эквивалентный
      диапазон) в отдельные child modules, не смешивая их с synthetic regressions.
- [x] 2.4 Сохранить текущие test function names / selectors и не менять acceptance semantics.

## 3. Validation

- [x] 3.1 Запустить `cargo test -p bsl-backend --bin bsl-lsp-server --no-run`.
- [x] 3.2 Прогнать representative targeted selectors из каждого вынесенного семейства и
      подтвердить selector parity после split.
- [x] 3.3 Подтвердить, что итоговая структура больше не содержит одного detached Rust test module
      масштаба `~60k LOC`.
- [x] 3.4 Запустить `openspec validate refactor-45-split-lsp-core-test-suite --strict
      --no-interactive`.
