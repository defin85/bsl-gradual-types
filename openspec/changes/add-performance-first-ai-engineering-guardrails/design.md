## Context
В текущем контуре уже есть сильные функциональные и latency-требования, особенно в `bsl-intellisense-v2`. Но для performance-critical AI-assisted разработки остаются системные риски:
- latency-гейт сам по себе не ловит регрессии по аллокациям и lock contention;
- архитектурные решения могут приниматься "по коду", а не через явный decision log;
- в pressure-ситуации возможен anti-pattern: подгонка тестов под текущую реализацию вместо исправления root cause.

Для non-MVP уровня нужен процесс, где архитектурное качество и производительность валидируются так же строго, как функциональная корректность.

## Goals / Non-Goals
- Goals:
  - Зафиксировать обязательный ADR/doc-first процесс для архитектурно-значимых/perf-critical изменений.
  - Сделать acceptance контур fail-closed: protected tests/contracts/perf baselines нельзя менять ad-hoc.
  - Расширить completion quality gate до ресурсных метрик (allocations + lock contention), а не только latency.
  - Добавить root-cause observability по resource pressure с низкой кардинальностью.
- Non-Goals:
  - Массовый lock-free rewrite всего runtime.
  - Снятие ответственности с code review.
  - Введение нестабильных/сложно воспроизводимых synthetic perf-тестов без baseline policy.

## Decisions
- Decision: Ввести двухуровневый guardrail: process level (`dev-workflow`) + runtime level (`bsl-intellisense-v2`).
  - Why: Один только process gate не ловит runtime регрессии, а один runtime gate не предотвращает архитектурный drift.

- Decision: Считать acceptance assets (ключевые тесты, versioned contracts, perf baselines) protected и immutable в рамках implementation change.
  - Why: Это устраняет класс ошибок "тесты подогнаны под код" и делает acceptance воспроизводимым.

- Decision: Расширить warm-path completion gates до `latency + allocations + lock contention`.
  - Why: Реальная деградация в Rust часто появляется как allocator churn/lock wait до видимого latency провала.

- Decision: Использовать только low-cardinality resource observability labels.
  - Why: Высокая кардинальность ломает эксплуатационную ценность метрик и усложняет сравнение baseline vs candidate.

## Alternatives Considered
- Альтернатива: Оставить только latency gates.
  - Rejected: скрытые регрессии по аллокациям и contention обнаруживаются слишком поздно.

- Альтернатива: Полагаться только на ручной ревью без формальных process gates.
  - Rejected: не масштабируется и не даёт детерминированного acceptance-контракта.

- Альтернатива: Сразу требовать тотальный lock-free дизайн.
  - Rejected: чрезмерно рискованно и дорого; нужен incremental подход с измеримыми бюджетами.

## Risks / Trade-offs
- Риск: рост времени на подготовку change из-за ADR/doc-first шага.
  - Mitigation: ограничить ADR только архитектурно-значимыми/perf-critical изменениями с явными триггерами.

- Риск: ложные срабатывания perf-gate из-за нестабильного окружения.
  - Mitigation: versioned baseline artifacts, фиксированные профили (`small/large/churn`), минимальный sample size и детерминированный отчёт.

- Риск: команды начнут обходить protected-assets policy.
  - Mitigation: fail-closed CI gate + отдельный approved change для обновления acceptance assets.

## Migration Plan
1. Утвердить change и spec deltas.
2. Внедрить process-gates (`ADR`, `doc-first`, `protected assets`) в workflow/CI.
3. Добавить resource instrumentation в completion hot path.
4. Включить расширенный perf-gate с baseline artifacts.
5. Провести staged rollout (warning-only -> blocking) и зафиксировать ownership.

## External References
- Rust Performance Book (profiling/allocation guidance): https://nnethercote.github.io/perf-book/profiling.html
- ADR guidance (decision log lifecycle): https://adr.github.io/
- AWS ADR process (accepted ADR immutability, supersede model): https://docs.aws.amazon.com/prescriptive-guidance/latest/architectural-decision-records/adr-process.html

## Open Questions
- Где хранить канонический protected-assets manifest: `contracts/**` или отдельный `workflow/` манифест?
- Какие стартовые budget thresholds принять для `allocations_per_completion` и `lock_wait_ms` до первого полного baseline цикла?
