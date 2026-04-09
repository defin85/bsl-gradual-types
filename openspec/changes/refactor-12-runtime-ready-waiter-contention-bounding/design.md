## Context
Сейчас runtime already умеет различать apply backlog и `wait_for_file_version` backlog в observability, но архитектурно readiness observation всё ещё входит в generic background FIFO слишком поздно.

В [runtime.rs](/home/egor/code/bsl-gradual-types/bsl-runtime/src/application/intellisense_v2/facade/runtime.rs) `WaitForFileVersion` сначала отправляется в background channel, и только после обработки writer thread request становится passive waiter в `waiters: HashMap<FileId, Vec<PendingWaiter>>`. Это означает:

- до регистрации wait request ничем не отличается от любого другого background command;
- unrelated `ApplyChanges` и другой background traffic могут задерживать не сам apply результата, а сам факт наблюдения за ним;
- completion и didSave follow-up получают лишний tail ещё до actual passive wait.

## Goals
- Сделать readiness observation low-latency и passive.
- Сохранить truthful separation между readiness wait и actual apply execution backlog.
- Удержать existing correctness contract для requested revision.
- Не увеличивать runtime parallelism и не строить полный scheduler rewrite.

## Non-Goals
- Не переписывать весь runtime transport на новую concurrency model.
- Не лечить cold semantic work этим change.
- Не отменять request-centric observability distinction между `wait_for_file_version`, `apply_lag` и `semantic_diagnostics_query`.

## Decisions

### 1. Регистрация wait должна обходить raw generic FIFO residency
Ключевой contract: readiness waiter должен становиться наблюдаемым быстро.

Это не означает bypass requested revision correctness. Это означает, что request:

- быстро регистрирует интерес к `(file_id, min_version)` в writer-owned readiness state;
- затем пассивно ждёт wake-up;
- не spend-ит seconds-scale queue wait только на пути к registration.

### 2. Passive waiting и actual apply backlog остаются разными failure classes
После change система должна truthfully отвечать на два разных вопроса:

- “сколько заняла регистрация passive waiter?”;
- “сколько заняло actual waiting for requested revision / apply execution?”.

Если эти классы смешать, acceptance снова не сможет различить architecture regression от honest apply lag.

### 3. Completion и didSave follow-up обязаны использовать один readiness contract
Этот change не должен создать два разных механизма readiness observation:

- один для completion;
- другой для didSave follow-up.

Оба path должны пользоваться одной writer-owned readiness surface, иначе regressions останутся workload-dependent и не будут воспроизводимы общим gate.

### 4. Bounded fail-closed semantics сохраняется
Change не обещает, что requested revision всегда станет ready быстро. Он обещает, что request не потеряет секунды до passive waiter registration.

Если apply реально задержан или revision не становится ready, request всё ещё может truthfully:

- timeout;
- вернуть empty/fail-closed outcome;
- опубликовать explicit apply-lag / readiness attribution.

### 5. Full writer transport rewrite explicitly deferred
Unbounded `std::sync::mpsc` transport остаётся потенциальным future hardening scope, но этот change intentionally narrower:

- сначала убираем логическую ошибку “passive wait registration inherits raw FIFO residency”;
- только потом, если evidence останется плохим, рассматриваем bounded transport migration как отдельный follow-up.

## Alternatives Considered

### A. Поднять quotas/permits
Rejected.

Это может замаскировать symptom, но не убирает сам registration-before-wait bottleneck.

### B. Перевести все runtime commands на новый bounded async transport сразу
Deferred.

Слишком широкий change для текущей incident-driven задачи. Он сложнее, рискованнее и тяжелее по rollout.

### C. Оставить waiter model как есть и лечить только apply execution
Rejected.

Тогда seconds-scale tail перед самой регистрацией wait останется, а acceptance снова будет смешивать registration latency и actual apply lag.

## Risks
- Если readiness side-channel будет ошибочно обходить writer-owned truth source, можно получить stale/incorrect ready observations.
- Если observability не разделит registration latency и actual wait latency, новый change потеряет diagnosability.
- Если разные paths получат разные readiness mechanisms, bug останется частично скрытым.

## Mitigations
- Оставить owner truth в writer-owned applied revision state.
- Явно тестировать completion и didSave follow-up одним readiness contract.
- Сохранять additive metrics/request-centric trace fields для registration vs actual wait separation.
