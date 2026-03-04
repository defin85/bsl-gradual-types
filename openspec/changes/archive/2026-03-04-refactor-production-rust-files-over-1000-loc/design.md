## Context
Change охватывает 28 production Rust файлов с размером >1000 LOC. Файлы распределены по нескольким подсистемам (`backend`, `bsl-runtime`, `analysis-v2`, `bsl-agent`, `semantic-diagnostics`, `bsl-repository`), поэтому требуется согласованный подход к декомпозиции с жёстким контролем поведения.

Ключевое ограничение от заказчика: refactor должен быть строго behavior-preserving.

## Goals / Non-Goals
- Goals:
  - Декомпозировать все production `.rs` файлы >1000 LOC до `<=1000 LOC`.
  - Довести целевые файлы до LLM-friendly budget: `<=800 LOC`, `<=80 KiB`, `<=12000 tokens (o200k_base)`.
  - Сохранить поведение без изменений (контракты, ответы, диагностики, completion, runtime semantics).
  - Вынести тесты из production файлов в отдельные test paths (запрет inline `#[cfg(test)] mod tests` в production scope).
  - Ввести повторяемый workflow для кампаний декомпозиции крупных файлов.
- Non-Goals:
  - Перепроектирование доменных алгоритмов.
  - Изменение внешних API/LSP контрактов.
  - Оптимизационные эксперименты, меняющие семантику.

## Constraints
- Scope только production Rust код:
  - include: `*.rs` в рабочих crate;
  - exclude: `third_party/**`, `**/target/**`, `**/node_modules/**`, `tests/benches/examples/fixtures/mocks`.
- LLM-readability budgets для target files кампании:
  - `LOC <= 800`;
  - `bytes <= 80 KiB`;
  - `tokens <= 12000` в `o200k_base`.
- Enforcement по бюджетам выполняется отдельным script-based gate (локально), без обязательного CI workflow на текущем этапе.
- Behavior-preserving:
  - одинаковые публичные ответы для одинаковых входов;
  - отсутствие intentional изменений в diagnosics/completion/API semantics.
- Декомпозиция должна идти через выделение модулей/подмодулей с явными границами ответственности, а не через перенос “как есть”.

## Architecture Decisions
- Decision: batch-based refactor по подсистемам, а не “весь репозиторий одним PR”.
  - Why: снижает риск регрессий и упрощает review/rollback.

- Decision: parity-first workflow для каждого batch.
  - Что фиксируется до правок:
    - target files batch;
    - baseline прогон тестов/контрактов;
    - инварианты поведения batch.
  - Why: поведение должно оставаться неизменным.

- Decision: enforce size gate (`<=1000 LOC`) как объективный критерий завершения.
  - Why: исключает “частично разрезали, но red-zone осталась”.

- Decision: enforce LLM-budget gate (`<=800 LOC`, `<=80 KiB`, `<=12000 o200k tokens`) для target files.
  - Why: ключевая цель change — чтобы LLM могла читать/редактировать файл целиком без упора в лимиты.

- Decision: inline test modules в production `.rs` запрещены; тесты выносятся в отдельные test paths.
  - Why: снижение шумового объёма production файлов и стабилизация LLM-прохода по коду.

## Refactor Strategy
1. Зафиксировать inventory и разбить на batch’и по подсистемам.
2. Для каждого batch:
   - выделить модульные границы (router/orchestrator/service/adapter/policy/helper);
   - вынести внутренние блоки в подмодули;
   - сохранить текущие публичные сигнатуры на внешней границе;
   - прогнать parity matrix.
3. После завершения всех batch:
   - проверить отсутствие production `.rs` >1000 LOC;
   - прогнать полный verification set.

## Batch Plan
1. LSP server and web handlers:
   - `backend/src/bin/lsp_server/server/core.rs`
   - `backend/src/bin/lsp_server/server/language_server.rs`
   - `backend/src/bin/lsp_server/server/completion_dispatcher.rs`
   - `backend/src/bin/lsp_server/handlers/references_and_rename.rs`
   - `backend/src/bin/lsp_server/handlers/completion.rs`
   - `backend/src/bin/lsp_server/commands/configuration.rs`
   - `backend/src/presentation/web/handlers.rs`
   - `backend/src/bin/intellisense_perf.rs`
   - `backend/src/perf_gate_evaluator.rs`
2. Runtime services and observability:
   - `bsl-runtime/src/system/basic_observability.rs`
   - `bsl-runtime/src/application/type_system/services/completion_service.rs`
   - `bsl-runtime/src/application/type_system/services/completion_ranking.rs`
   - `bsl-runtime/src/application/intellisense_v2/facade.rs`
   - `bsl-runtime/src/application/intellisense_v2/policy.rs`
3. Runtime coordinator and loaders:
   - `bsl-runtime/src/system/system_coordinator/config_loader.rs`
   - `bsl-runtime/src/system/system_coordinator/lifecycle.rs`
   - `bsl-runtime/src/system/system_coordinator/coordinator.rs`
   - `bsl-runtime/src/system/disk_cache.rs`
   - `bsl-runtime/src/system/runtime_config.rs`
   - `bsl-runtime/src/system/parser_coordinator.rs`
   - `bsl-runtime/src/data/loaders/config_metadata_parser/discovery.rs`
   - `bsl-runtime/src/data/loaders/config_metadata_parser/converter.rs`
4. Analysis and semantic layers:
   - `analysis-v2/src/lib.rs`
   - `analysis-v2/src/type_inference_v2.rs`
   - `semantic-diagnostics/src/visitor.rs`
   - `bsl-repository/src/repository.rs`
5. Agent layer:
   - `bsl-agent/src/session/mod.rs`
   - `bsl-agent/src/server/mod.rs`

## Validation Matrix (behavior-preserving)
- LLM/readability gate:
  - `python3 scripts/check-rust-file-llm-budget.py` (или эквивалентный специальный script-based gate)
- Compilation/lint:
  - `cargo fmt --all -- --check`
  - `cargo clippy --workspace --all-targets -- -D warnings`
- Core tests:
  - `cargo test --workspace --locked`
- LSP/IntelliSense regression suites (existing tests/scripts)
- MCP agent integration tests (existing tests/scripts)
- Existing contract/perf gates без ослабления acceptance assets

## Risks / Trade-offs
- Риск: слишком крупный batch увеличит time-to-review.
  - Mitigation: дробить batch до reviewable объёма.
- Риск: “скрытая” behavioral регрессия при переносе логики между модулями.
  - Mitigation: parity matrix до/после и запрет изменения acceptance assets.
- Риск: формальное снижение LOC без реального снижения связности.
  - Mitigation: требовать явные module responsibilities в каждом batch.
- Риск: file укладывается в LOC, но остаётся тяжёлым по токенам/байтам для LLM.
  - Mitigation: обязательный multi-metric budget gate (LOC + bytes + tokens).

## Rollback
- Rollback на уровне batch/PR: откат последнего batch без затрагивания остальных.
- Если parity не подтверждается, batch не мержится.
