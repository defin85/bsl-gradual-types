## Context
В текущем контуре уже есть сильные функциональные и latency-требования, особенно в `bsl-intellisense-v2`. Но для performance-critical AI-assisted разработки остаются системные риски:
- latency-гейт сам по себе не ловит регрессии по аллокациям и lock contention;
- архитектурные решения могут приниматься "по коду", а не через явный decision log;
- в pressure-ситуации возможен anti-pattern: подгонка тестов под текущую реализацию вместо исправления root cause.
- логика perf-gate может размазываться между CI скриптами и runtime entrypoints, что приводит к drift в порогах и verdict.

Для non-MVP уровня нужен процесс, где архитектурное качество и производительность валидируются так же строго, как функциональная корректность.

Текущие perf-исследования p31 (large_warm/churn) дополнительно показали устойчивый паттерн:
- критический вклад в latency даёт не build индекса, а длительное ожидание до первого `WillExecute(type_index)`;
- в проблемном режиме parse/build занимают миллисекунды, тогда как pre-execution gap может занимать десятки секунд.

Это подтверждает, что process и gate-контур должен учитывать lock/contention/queue/resource сигналы alongside latency.

## Goals / Non-Goals
- Goals:
  - Зафиксировать обязательный ADR/doc-first процесс для архитектурно-значимых/perf-critical изменений.
  - Сделать acceptance контур fail-closed: protected tests/contracts/perf baselines нельзя менять ad-hoc.
  - Расширить completion quality gate до ресурсных метрик (allocations + lock contention), а не только latency.
  - Добавить root-cause observability по resource pressure с низкой кардинальностью.
  - Зафиксировать `Option B` как единственную архитектуру perf-gate: dedicated evaluator module + versioned schema contract.
- Non-Goals:
  - Массовый lock-free rewrite всего runtime.
  - Снятие ответственности с code review.
  - Введение нестабильных/сложно воспроизводимых synthetic perf-тестов без baseline policy.
  - Поддержка нескольких конкурирующих реализаций perf-gate (inline в core + скриптовые копии логики).

## Decisions
- Decision: Ввести двухуровневый guardrail: process level (`dev-workflow`) + runtime level (`bsl-intellisense-v2`).
  - Why: Один только process gate не ловит runtime регрессии, а один runtime gate не предотвращает архитектурный drift.

- Decision: Принять `Option B` как единственный путь реализации perf-gate.
  - What:
    - perf verdict вычисляется только в dedicated perf-gate module;
    - consumers (CI/harness/runtime checks) вызывают один и тот же evaluator API;
    - дублирование порогов/правил в скриптах и `lsp_server` core запрещено.
  - Why: единый evaluator устраняет drift в логике и дает воспроизводимый verdict.

- Decision: Зафиксировать schema contract как source of truth для perf-gate input/baseline/report.
  - What:
    - versioned schema хранится в `contracts/intellisense-perf-gate/v1/**`;
    - минимум: input schema, baseline schema, report schema;
    - report MUST включать `contract_version`, `verdict`, `reason_codes`, и профиль (`small|large|churn`).
  - Why: формальный контракт делает проверку детерминированной и проверяемой через compatibility-diff.

- Decision: Считать acceptance assets (ключевые тесты, versioned contracts, perf baselines) protected и immutable в рамках implementation change.
  - Why: Это устраняет класс ошибок "тесты подогнаны под код" и делает acceptance воспроизводимым.

- Decision: Расширить warm-path completion gates до `latency + allocations + lock contention`.
  - Why: Реальная деградация в Rust часто появляется как allocator churn/lock wait до видимого latency провала.

- Decision: Использовать только low-cardinality resource observability labels.
  - Why: Высокая кардинальность ломает эксплуатационную ценность метрик и усложняет сравнение baseline vs candidate.

- Decision: Классификация `change_criticality` обязательна и machine-readable до запуска process-gates.
  - What:
    - `change_criticality` ограничен enum: `routine`, `behavioral`, `architectural`, `perf_critical`;
    - ADR/doc-first/perf gates обязательны для `architectural|perf_critical`;
    - отсутствие/невалидность классификации трактуется fail-closed.
  - Why: убирает неявные трактовки и спорные решения "подпадает/не подпадает".

- Decision: Test-first evidence фиксируется в machine-readable контракте.
  - What:
    - evidence содержит минимум `failing_ref`, `passing_ref`, `scope`, `change_id`;
    - gate валидирует evidence schema и связь с change scope.
  - Why: превращает test-first из декларации в автоматизируемый и проверяемый контракт.

- Decision: Bootstrap policy initial budgets формализована и обязательна до blocking mode.
  - What:
    - initial budgets вычисляются на репрезентативных профилях (`small|large|churn`) по фиксированной методике;
    - sample size: минимум 5 прогонов на профиль;
    - aggregation: median от profile-level p95 (и p99 для latency ceilings);
    - budget фиксируется в versioned contract и утверждается ADR до включения blocking gate.
  - Why: убирает недетерминированность первого включения fail-closed perf gate.

- Decision: Canonical metric keys для resource gate фиксируются без эквивалентов в рамках major версии.
  - What:
    - обязательные keys: `allocations_per_completion`, `allocated_bytes_per_completion`,
      `lock_wait_ms_per_completion`, `lock_contention_events_per_completion`;
    - отсутствие любого key -> fail (`missing_required_metric_field`).
  - Why: предотвращает schema drift между evaluator и consumers.

## Concrete Contracts (Tasks 2.1-2.9)
### 2.1 ADR template and classification criteria
ADR обязателен для `change_criticality in {architectural, perf_critical}`.

Минимальный шаблон ADR:
1. `Title` и `Date`
2. `Status` (`proposed|accepted|superseded`)
3. `Change ID` + `change_criticality`
4. `Context` (проблема, ограничения)
5. `Options Considered` (минимум 2)
6. `Decision` и `Rationale`
7. `Budgets` (`latency`, `allocations`, `lock contention`)
8. `Validation Plan` (tests/perf evidence)
9. `Rollback / Supersede Plan`
10. `Owners and Approvers`

Критерии `architecturally significant/perf-critical`:
- меняется модель синхронизации hot path;
- добавляются/меняются process-global locks;
- меняются cache topology или consistency границы;
- меняется perf-gate evaluator/schema contract;
- меняются SLO/budget ceilings.

### 2.2 Protected-assets manifest
Protected-assets v1 (immutable в implementation change):
- `contracts/intellisense-perf-gate/**`
- `openspec/specs/dev-workflow/spec.md`
- `openspec/specs/bsl-intellisense-v2/spec.md`
- `backend/src/bin/lsp_server/server/core.rs` (perf gate report/verdict path)
- `backend/src/bin/intellisense_perf.rs`
- `tests/perf/**`

Нарушение без отдельного approved change -> fail reason:
- `protected_acceptance_asset_modified`.

### 2.3 Schema contract v1 and format/version policy
`Option B` contract root:
- `contracts/intellisense-perf-gate/v1/input.schema.json`
- `contracts/intellisense-perf-gate/v1/baseline.schema.json`
- `contracts/intellisense-perf-gate/v1/report.schema.json`

Version policy:
- backward-compatible additive change: same major (`v1`);
- breaking change: mandatory new major (`v2`) + migration note;
- compatibility-diff gate обязателен для всех изменений schema.

Required report fields:
- `contract_version`
- `verdict` (`pass|fail`)
- `reason_codes[]`
- `profiles.small|large|churn`

### 2.4 Ownership model
Ownership фиксируется по роли (не по персоналии):
- `ADR Owner`: архитектурная группа backend/runtime.
- `Perf Budget Owner`: владелец `bsl-intellisense-v2` perf-SLO.
- `Protected Assets Owner`: владелец `dev-workflow` и CI policy.
- `Contract Owner`: владелец `contracts/intellisense-perf-gate/*`.

Approval policy:
- ADR acceptance требует `ADR Owner + Perf Budget Owner`.
- Contract version bump требует `Contract Owner + Protected Assets Owner`.

### 2.5 Absolute latency ceilings (approved initial values)
Начальные абсолютные ceilings для warm-path completion:

| Profile | p95 ceiling | p99 ceiling |
| --- | --- | --- |
| `small` | `300ms` | `600ms` |
| `large` | `1500ms` | `3000ms` |
| `churn` | `1800ms` | `3500ms` |

Дополнительно к absolute ceilings всегда применяется relative ratio gate к versioned baseline.
Изменение ceilings возможно только через ADR + update baseline contract.

### 2.6 Dedicated perf-gate module boundary
Единая boundary:
- module: `backend/src/bin/lsp_server/server/perf_gate_evaluator.rs` (или эквивалентный выделенный модуль runtime/gate)
- API:
  - `evaluate(input, baseline) -> report`
  - `validate_contract_version(input, baseline)`
- Consumers:
  - CI gate
  - local perf harness
  - runtime acceptance check

Reason-code taxonomy (v1):
- `missing_required_metric_field`
- `unsupported_contract_version`
- `latency_relative_ratio_exceeded`
- `latency_absolute_ceiling_exceeded`
- `allocation_budget_exceeded`
- `lock_wait_budget_exceeded`
- `lock_contention_budget_exceeded`
- `protected_acceptance_asset_modified`
- `change_criticality_missing_or_unknown`
- `test_first_evidence_missing_or_invalid`
- `initial_budget_not_fixed`
- `perf_gate_architecture_violation`

### 2.7 `change_criticality` schema (machine-readable)
```json
{
  "schema_version": "v1",
  "change_id": "add-performance-first-ai-engineering-guardrails",
  "change_criticality": "perf_critical",
  "rule_id": "criticality.rules.v1/perf_hot_path",
  "reason": "Touches interactive completion hot path and perf-gate architecture"
}
```
Enum is fixed: `routine|behavioral|architectural|perf_critical`.

### 2.8 Test-first evidence schema
```json
{
  "schema_version": "v1",
  "change_id": "add-performance-first-ai-engineering-guardrails",
  "scope": "backend/runtime",
  "failing_ref": "path-or-ci-run-before",
  "passing_ref": "path-or-ci-run-after",
  "reason_codes": []
}
```
Validation fails если любой из `change_id|scope|failing_ref|passing_ref` отсутствует.

### 2.9 Bootstrap methodology for initial budgets
Обязательная методика:
1. Profiles: `small|large|churn`.
2. Для каждого профиля минимум `N=5` валидных прогонов.
3. Aggregation:
   - budget `p95` = median(profile-level `p95`);
   - budget `p99` = median(profile-level `p99`).
4. Resource budgets рассчитываются аналогично (median per-profile).
5. Результат фиксируется в versioned baseline contract.
6. Blocking mode включается только после ADR approval и зафиксированного baseline.

## Alternatives Considered
- Альтернатива A: Inline/per-script perf gate (логика распределена между `lsp_server` core и shell/Python helpers).
  - Rejected: пороги и reason-codes расходятся между entrypoints; сложно доказать единый fail-closed verdict.

- Альтернатива C: Только внешний скриптовый gate без выделенного Rust evaluator module.
  - Rejected: слабый compile-time контракт, высокий риск schema drift и расхождения с runtime поведением.

- Альтернатива: Полагаться только на ручной ревью без формальных process gates.
  - Rejected: не масштабируется и не даёт детерминированного acceptance-контракта.

- Альтернатива: Сразу требовать тотальный lock-free дизайн.
  - Rejected: чрезмерно рискованно и дорого; нужен incremental подход с измеримыми бюджетами.

## Risks / Trade-offs
- Риск: рост времени на подготовку change из-за ADR/doc-first шага.
  - Mitigation: ограничить ADR только архитектурно-значимыми/perf-critical изменениями с явными триггерами.

- Риск: ложные срабатывания perf-gate из-за нестабильного окружения.
  - Mitigation: versioned baseline artifacts, фиксированные профили (`small/large/churn`), минимальный sample size и детерминированный отчёт.

- Риск: стоимость первичного выделения dedicated module и schema миграции выше, чем локальный inline patch.
  - Mitigation: staged integration в существующие entrypoints без изменения product behavior на первом шаге (только унификация evaluator).

- Риск: команды начнут обходить protected-assets policy.
  - Mitigation: fail-closed CI gate + отдельный approved change для обновления acceptance assets.

## Migration Plan
1. Утвердить change и spec deltas.
2. Зафиксировать schema contract v1 (`contracts/intellisense-perf-gate/v1/**`) и compatibility policy.
3. Выделить dedicated perf-gate evaluator module с единым API (`input + baseline -> report`).
4. Перевести все consumers (CI/harness/runtime checks) на вызов evaluator module и schema contract.
5. Внедрить process-gates (`ADR`, `doc-first`, `protected assets`) в workflow/CI.
6. Добавить resource instrumentation в completion hot path и включить blocking perf-gate по unified report.
7. Зафиксировать ownership (module owner, contract owner, budget owner) и runbook изменения порогов через ADR.

## External References
- Rust Performance Book (profiling/allocation guidance): https://nnethercote.github.io/perf-book/profiling.html
- ADR guidance (decision log lifecycle): https://adr.github.io/
- AWS ADR process (accepted ADR immutability, supersede model): https://docs.aws.amazon.com/prescriptive-guidance/latest/architectural-decision-records/adr-process.html

## Open Questions
- Нужен ли отдельный профиль `steady_parallel` (кроме `small|large|churn`) для фиксации process-global lock regressions в burst-сценариях?
