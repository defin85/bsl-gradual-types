## Контекст
После `refactor-current-revision-readiness-fast-lane`, `refactor-completion-prepare-lightweight-exact-split` и `refactor-document-symbol-interactive-isolation` основной completion bottleneck снова сместился.

Incident bundle `2026-03-24T18:21:45Z` показывает другой root cause:
- request `42` имеет `service_future_to_first_poll_wait_ms=5903`, но `server_handler_exec_ms=120`;
- у того же request `wait_for_file_version_runtime.resolution = immediate` и `wait_elapsed_ms = 0`;
- request `32` уже first-polled за `1ms`, но продолжает жить в handler `8961ms`;
- второй same-file probe стартует при `active_completion_count_at_start=1`.

Практический смысл: новый completion request ждёт не current-revision readiness и не auxiliary outline traffic, а stale active completion, который уже потерял ценность, но ещё удерживает active turn.

Текущий код уже делает часть нужной работы:
- dispatcher proactively cancel-ит older active completion при приходе нового same-file request;
- request-level cancellation registry существует;
- completion pipeline имеет checkpoints до `response_build` и после него.

Но этого недостаточно: длинный `response_build` tail выполняется слишком крупным куском, а active turn освобождается только после полного unwind request scope. Поэтому superseded request всё ещё может держать интерактивный slot слишком долго.

## Цели
- Гарантировать, что newer same-file completion не копит seconds-scale pre-poll wait только из-за stale active completion, который уже потерял latest-wins.
- Сделать superseded/cancelled completion кооперативно прерываемым внутри длинного `response_build` tail.
- Доказать новый контракт live overlap acceptance test и representative real-module gate, а не только synthetic cancellation smoke.

## Не-цели
- Не менять strict current-revision / fail-closed semantics completion.
- Не открывать новый change по `documentSymbol`, diagnostics или другим non-completion bottleneck-классам.
- Не вводить новый stale/degraded fallback как замену prompt cancellation.
- Не превращать этот change в общий executor/concurrency redesign всего LSP server.

## Решение

### 1. Root cause фиксируется как active-turn retention, а не как generic cancel outcome
Недостаточно знать, что старый request в итоге завершится `cancelled` или `incomplete_empty`.

Новый observable contract:
- если same-file completion уже first-polled и затем потерял latest-wins из-за нового completion или explicit cancel,
- он MUST перестать считаться владельцем active interactive turn не позже ближайшего cooperative checkpoint после того, как supersession/cancel стал наблюдаемым,
- и MUST NOT удерживать newer request в seconds-scale `service_future_created -> first poll`.

То есть change про prompt release ownership, а не только про итоговый bounded outcome.

### 1.1 Зафиксированное архитектурное ограничение
После архитектурного review для этого change принято явное ограничение реализации:
- fix MUST оставаться на existing completion path;
- fix MUST устранять prompt release stale active completion;
- новый admission workaround, отдельная transport lane, увеличение concurrency само по себе или общий executor/scheduler redesign НЕ считаются допустимой реализацией этого change.

Причина: incident и текущий код указывают на локальный gap между proactive cancel и release active ownership внутри уже существующего completion pipeline. Если лечить это только admission-обходом, stale работа всё равно продолжит съедать CPU/runtime budget и change потеряет связь с проверяемым root cause.

### 2. Кооперативная cancellation должна существовать внутри response-build tail
Сейчас coarse checkpoints вокруг большого `response_build` блока не гарантируют своевременный выход: stale request может дойти до тяжёлого `collect/rank/format` и только потом проверить cancel state.

Поэтому implementation должен обеспечить одно из двух:
- либо явные checkpoints внутри `collect`, `rank`, `format` и смежных тяжёлых completion стадий;
- либо эквивалентную interruptible boundary, которая даёт тот же observable result: superseded request быстро перестаёт удерживать active turn и не дожёвывает много секунд stale работы.

OpenSpec здесь фиксирует именно observable contract, а не конкретную форму refactor-а.

### 3. Prompt release лучше чинить в корне, а не маскировать concurrency workaround
Просто увеличить concurrency или отпустить guard раньше без реального interruption недостаточно:
- CPU и runtime queue всё равно продолжат тратить время на бессмысленный stale request;
- следующая регрессия может остаться скрытой, если slot формально отпущен, но hot path всё ещё съедается старой работой.

Поэтому предпочтительный путь:
- cooperative cancellation внутри response-build tail;
- bounded stop stale request;
- release active ownership как следствие реального прекращения stale critical path, а не чисто косметического bookkeeping.

### 4. Acceptance обязан мерить same-file overlap, а не только cancel/no-late-publish
Существующие cancellation tests доказывают, что request можно отменить и не публиковать поздний ответ. Они не доказывают главное для этого инцидента: второй same-file completion достигает first poll вовремя, пока первый request уже успел войти в handler.

Нужен новый acceptance layer:
- live LSP overlap regression с двумя same-file completion request;
- старый request должен завершиться boundedly после supersession;
- новый request должен укладываться в first-poll budget;
- representative real-module gate должен воспроизводить этот overlap профиль на большом реальном модуле.

## Рассмотренные альтернативы

### Освобождать active turn только bookkeeping-ом, не прерывая stale работу
Отклонено. Это скрывает симптом на уровне счётчика, но не убирает бессмысленную CPU/runtime нагрузку и может оставить latency regression в другом месте.

### Просто поднять concurrency / slots
Отклонено. Это pressure relief, а не устранение root cause. Superseded request всё равно продолжит занимать ресурсы после потери актуальности.

### Добавить новую admission lane вместо исправления existing completion path
Отклонено. Для этого change это architectural bypass, а не root-cause fix. Он может уменьшить pre-poll stall локально, но не гарантирует prompt release stale active completion внутри уже начатого `response_build`.

### Вернуть fallback на stale completion, если новый request ждёт слишком долго
Отклонено. Это нарушает уже принятый strict current-revision contract и маскирует defect вместо его исправления.

## Риски и trade-offs

### Риск: дополнительные checkpoints увеличат hot-path overhead
Допустимо, если checkpoints остаются локальными для тяжёлых стадий и убирают multi-second stalls. Здесь correctness of interactive latency важнее микроскопического выигрыша на happy path.

### Риск: overlap gate окажется flaky
Смягчение:
- использовать детерминированный same-file harness;
- опираться на server-side first-poll attribution и bounded cancel outcome, а не только на wall-clock клиента;
- держать отдельный representative profile для real module.

### Риск: change начнёт смешиваться с более широким executor redesign
Смягчение:
- scope change ограничен same-file active completion supersession;
- proposal не требует общего redesign для всех interactive методов;
- всё, что выходит за пределы completion overlap contract, остаётся отдельным follow-up.

## Порядок внедрения
1. Добавить red regression на overlapping same-file completion с first-poll budget для нового request.
2. Внести cooperative cancellation / active-turn release в completion response-build path.
3. Расширить representative real-module gate overlap profile и checked-in evidence.
4. Обновить docs/runbook только после того, как overlap path станет частью shipped verification.
