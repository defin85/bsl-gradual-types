## Context

`backend/src/bin/lsp_server/server/core/tests.rs` сейчас имеет `59_777 LOC` и содержит:

- `238` test cases (`190` async/tokio tests и `48` plain tests);
- `414` functions total;
- длинный shared harness layer в начале файла;
- несколько тематически разных семейств тестов, включая synthetic regressions и live probes.

При этом production wiring уже позволяет безопасный split:

- `backend/src/bin/lsp_server/server/core.rs` подключает test module через `mod tests;`;
- рядом уже существует директория `backend/src/bin/lsp_server/server/core/`, так что directory
  module `core/tests/mod.rs` естественно ложится в текущую структуру.

Главный практический риск здесь не в compile time, а в сопровождаемости: локальные изменения в
acceptance tests начинают конфликтовать друг с другом просто из-за общей точки записи.

## Goals / Non-Goals

- Goals:
  - разрезать catastrophic detached test-suite outlier на reviewable themed modules;
  - сохранить текущие test selectors и способ запуска таргетных `cargo test ... <selector>`;
  - выделить shared harness/helpers в отдельный support layer;
  - отделить heavy live/report probes от synthetic/core regressions.
- Non-Goals:
  - менять production semantics;
  - менять названия acceptance probes без отдельного approved change;
  - превращать этот change в новую repo-wide кампанию для всех test files средней величины.

## Decisions

### 1. Сохранить внешний test-module hook стабильным

`core.rs` продолжит использовать `mod tests;`.

Рефакторинг должен происходить через замену плоского файла `core/tests.rs` на directory module
`core/tests/mod.rs`, чтобы production code wiring почти не менялся.

### 2. Вынести shared harness в `support.rs`

Общие imports, transport harness, helper assertions и utility builders должны жить в
`core/tests/support.rs` или эквивалентном support module.

Это уменьшает дублирование и позволяет тематическим test files импортировать только нужный
минимум.

### 3. Резать по семействам поведения, а не по произвольным line ranges

Целевые child modules должны группироваться по тестовой ответственности. Ожидаемые семейства:

- startup / transport / basic orchestration;
- `didSave` follow-up и supersession;
- diagnostics-save timeline;
- completion / current-context / readiness waits;
- live report probes.

Точные имена файлов могут отличаться, если итоговое разбиение остаётся последовательным и
reviewable.

### 4. Стабильность test selectors обязательна

Имена существующих test functions должны сохраняться.

Это критично, потому что текущие workflow и OpenSpec evidence опираются на таргетные вызовы вида
`cargo test -p bsl-backend --bin bsl-lsp-server p56_real_conf_big_... -- --nocapture`.

### 5. Бюджеты у результата должны быть reviewable, а не формальными

Этот change не вводит repo-wide жёсткий LOC gate для всех test files средней величины. Но для
целевого catastrophic suite он требует reviewable decomposition:

- themed child module SHOULD стремиться к `<=3000 LOC`;
- shared support module SHOULD оставаться существенно меньше исходного плоского файла;
- heavy live/report probes SHOULD быть отделены от synthetic/core regressions.

Если какое-то семейство всё ещё слишком велико, его нужно дробить дальше, а не переносить
монолит в новый файл.

## Alternatives Considered

### 1. Оставить файл как есть и ограничиться “аккуратнее редактировать”

Rejected.

При `59_777 LOC` проблема уже структурная. Осторожность редактора не устраняет conflict surface и
не делает review дешевле.

### 2. Резать purely by line count

Rejected.

Файлы “по 3000 строк каждая” без смысловых границ ухудшат навигацию и быстро вернут ту же
проблему, только в нескольких местах.

### 3. Переименовать тесты по новой файловой структуре

Rejected.

Это сломает текущие selector-based команды и acceptance evidence без реальной необходимости.

## Validation Strategy

- Подтвердить, что `core/tests.rs` заменён на directory module без изменения production behavior.
- Прогнать `cargo test -p bsl-backend --bin bsl-lsp-server --no-run`.
- Прогнать representative targeted selectors из каждого вынесенного семейства, включая:
  - diagnostics-save timeline;
  - same-version follow-up / current-context waits;
  - live `p45..p56` family или семантически эквивалентные representative probes.
- Подтвердить, что старые selector names по-прежнему invokable.
- Подтвердить, что в итоговой структуре больше нет одного плоского detached test module масштаба
  `~60k LOC`.

## Implementation Outcome

Итоговый split landed как directory module с тонкими top-level wrappers и include-based
decomposition внутри каждого крупного семейства:

- `backend/src/bin/lsp_server/server/core/tests/mod.rs` (`72 LOC`);
- `backend/src/bin/lsp_server/server/core/tests/support.rs` (`1102 LOC`);
- `backend/src/bin/lsp_server/server/core/tests/current_context_and_scale.rs` (`1791 LOC`);
- `backend/src/bin/lsp_server/server/core/tests/{startup_and_fastlane,current_revision_head,diagnostics_save_timeline,did_save_followup,interactive_completion,lsp_features_and_observability,snapshot_status_and_perf,live_reports}.rs`
  теперь wrapper files по `6-7 LOC`;
- shared root fragments живут в `backend/src/bin/lsp_server/server/core/tests/root/*.rs`;
- themed child fragments живут в `backend/src/bin/lsp_server/server/core/tests/<family>/*.rs`.

После second-pass split ни один Rust test fragment под `core/tests/**` больше не превышает
`2894 LOC`; крупнейший файл сейчас —
`backend/src/bin/lsp_server/server/core/tests/interactive_completion/precompute_and_bounded_fail_closed.rs`.

`backend/src/bin/lsp_server/server/core.rs` сохранил стабильный внешний hook через
`#[path = "core/tests/mod.rs"] mod tests;`, поэтому production wiring не менялся.

## Validation Notes

- `cargo test -p bsl-backend --bin bsl-lsp-server --no-run` проходит на итоговой структуре.
- Representative selector parity повторно подтверждена после second-pass split таргетными
  вызовами из root module и каждого rewrapped family, включая live probe
  `p50_real_conf_big_ready_snapshot_phase_report_live`.
- Для `did_save_followup` тяжёлые save-followup probes на текущем дереве уже затронуты отдельным
  runtime residual и не используются как acceptance gate этого refactor. Selector parity для этого
  child module подтверждена через `p6_diagnostics_save_timeline_duration_to_nonzero_ms_filters_sub_ms_values`
  с сохранённым старым именем теста и новым путём `server::core::tests::did_save_followup::*`.
