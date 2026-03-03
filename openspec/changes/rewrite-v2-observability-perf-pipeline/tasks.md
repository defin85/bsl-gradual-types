## 1. Specification
- [ ] 1.1 Добавить в `bsl-intellisense-v2` requirements для полного rewrite observability/perf pipeline (`v3` boundary).
- [ ] 1.2 Зафиксировать requirements для centralized instrumentation API и запрета adapter-local bypass.
- [ ] 1.3 Зафиксировать requirements для registry-compiled projection completeness.
- [ ] 1.4 Зафиксировать requirements для fail-closed provenance/perf evidence acceptance.
- [ ] 1.5 Зафиксировать rollout requirements (dual-write/canary/rollback/legacy deprecation).

## 2. Architecture And Contracts
- [ ] 2.1 Утвердить целевую v3 архитектуру (`ingest -> validate -> project -> export`) как единую runtime boundary.
- [ ] 2.2 Зафиксировать ownership-модель: где допустим emission, где only materialization, где only export.
- [ ] 2.3 Спроектировать versioning/migration policy для `observability-completion-v2` и `intellisense-perf-gate`.
- [ ] 2.4 Зафиксировать provenance envelope (`change_id`, `generated_at`, `profile`, `schema_version`, `contract_version`) и fail-closed validator expectations.
- [ ] 2.5 Утвердить rollback policy при canary/parity drift.

## 3. Implementation
- [ ] 3.1 Ввести новый pipeline core module для canonical event ingestion + schema validation.
- [ ] 3.2 Реализовать registry compiler/materializer для deterministic projection в drilldown/legacy.
- [ ] 3.3 Перевести LSP/web/MCP/runtime emission на centralized instrumentation API.
- [ ] 3.4 Убрать прямые adapter-local публикации метрик в обход pipeline.
- [ ] 3.5 Перевести perf report generation на provenance envelope из invocation context (без hardcoded change id).
- [ ] 3.6 Обновить evaluator/gate path на fail-closed rules для provenance mismatch/missing.
- [ ] 3.7 Обновить versioned contracts/changelog/migration notes.

## 4. Rollout And Validation
- [ ] 4.1 Добавить contract/parity tests на registry completeness и projection determinism.
- [ ] 4.2 Добавить integration tests на unified serve-outcome emission для `completion/hover/signatureHelp/definition`.
- [ ] 4.3 Добавить tests на fail-closed provenance validation и rejection foreign artifacts.
- [ ] 4.4 Добавить canary/rollback tests и acceptance criteria для dual-write phase.
- [ ] 4.5 Прогнать versioned contracts checks и целевые perf gate validation сценарии.
- [ ] 4.6 `openspec validate rewrite-v2-observability-perf-pipeline --strict --no-interactive`.

## Dependencies / Parallelism
- [ ] D1 Пункты 1.1-1.5 и 2.1-2.5 блокируют 3.1-3.7.
- [ ] D2 Пункты 3.1 и 3.2 выполняются последовательно (materializer зависит от core model).
- [ ] D3 Пункты 3.3 и 3.5 можно выполнять параллельно после 3.1-3.2.
- [ ] D4 Пункты 3.6-3.7 зависят от 3.5.
- [ ] D5 Пункты 4.1-4.5 зависят от 3.1-3.7.
