## Context
В текущем контуре уже есть сильные функциональные и latency-требования, особенно в `bsl-intellisense-v2`. Но для performance-critical AI-assisted разработки остаются системные риски:
- latency-гейт сам по себе не ловит регрессии по аллокациям и lock contention;
- архитектурные решения могут приниматься "по коду", а не через явный decision log;
- в pressure-ситуации возможен anti-pattern: подгонка тестов под текущую реализацию вместо исправления root cause.
- логика perf-gate может размазываться между CI скриптами и runtime entrypoints, что приводит к drift в порогах и verdict.

Для non-MVP уровня нужен процесс, где архитектурное качество и производительность валидируются так же строго, как функциональная корректность.

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
- Какие стартовые budget thresholds принять для `allocations_per_completion` и `lock_wait_ms` до первого полного baseline цикла?
