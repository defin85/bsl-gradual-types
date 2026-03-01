# Change: Performance-first инженерный контур для AI-изменений (non-MVP)

## Why
Проект уже работает в performance-critical среде (Rust, явное владение ресурсами, строгие типы), где "просто компилируется" недостаточно. Для таких систем основная стоимость возникает не в первичной реализации, а в позднем исправлении архитектурных ошибок: лишних аллокаций в hot path, избыточных mutex/Arc, и рассинхрона между intended design и фактическим кодом.

При AI-assisted разработке этот риск усиливается: рабочий, но архитектурно слабый код может проходить функциональные тесты, а затем требовать дорогого переписывания.

Нужен fail-closed процессный и технический контракт, который фиксирует:
- архитектурные решения до кода;
- неизменяемость acceptance-артефактов во время имплементации;
- обязательные perf-доказательства не только по latency, но и по allocations/lock contention.

## What Changes
- **ADDED (dev-workflow)**: обязательный ADR gate для архитектурно-значимых/perf-critical изменений до начала реализации.
- **ADDED (dev-workflow)**: doc-first non-MVP контракт (proposal/design/tasks/spec deltas + acceptance matrix) как обязательное условие для реализации.
- **ADDED (dev-workflow)**: test-first цикл для backend/runtime behavioral changes с запретом ad-hoc изменений protected acceptance assets.
- **ADDED (dev-workflow)**: merge-gate с обязательными perf evidence артефактами (`latency`, `allocations`, `lock contention`) и fail-closed политикой.
- **ADDED (dev-workflow)**: `Option B` зафиксирован как единственный допустимый путь для perf-gate: dedicated perf-gate module + versioned schema contract (`contracts/intellisense-perf-gate/v1/**`) для input/baseline/report.
- **ADDED (bsl-intellisense-v2)**: resource budgets для интерактивного completion (alloc/lock alongside latency).
- **ADDED (bsl-intellisense-v2)**: low-cardinality observability контракт для root-cause по allocator/lock pressure.

## Impact
- Affected specs:
  - `dev-workflow`
  - `bsl-intellisense-v2`
- Affected code (implementation follow-up):
  - `analysis-v2/**` (completion hot path instrumentation)
  - `bsl-runtime/src/system/basic_observability.rs`
  - `backend/src/bin/lsp_server/**` и `backend/src/bin/intellisense_perf.rs` (интеграция с dedicated perf-gate module)
  - отдельный модуль perf gate evaluator (выделенная граница ответственности)
  - versioned contract artifacts в `contracts/intellisense-perf-gate/v1/**`
  - perf/benchmark harness и baseline artifacts в `tests/perf/**`
  - CI workflows и helper scripts для protected-assets/perf gates

## Non-Goals
- Переписывание всей runtime архитектуры в рамках одного change.
- Автоматическая "магическая" оптимизация всех hot paths без явных budget/measurement контрактов.
- Отмена инженерного review: change усиливает его, а не заменяет.
- Поддержка альтернативных архитектур perf-gate (inline/per-script дублирование логики вместо dedicated module + schema contract).
