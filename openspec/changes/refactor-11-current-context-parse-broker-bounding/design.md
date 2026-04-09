## Context
Свежий incident bundle показал, что `bsl.getCurrentContext` уже не портит truthful transport seams completion request'ов, но всё ещё создаёт disproportionate background CPU pressure:

- current-context parse source почти всегда идёт через `parser_coordinator`, а не через ready snapshot;
- parser-coordinator parse/wall tails измеряются десятками секунд;
- follower requests не повторяют parse body, но всё равно занимают scarce blocking CPU capacity, потому что duplicate suppression включается только после входа в blocking section.

Текущий путь в [command_handlers.rs](/home/egor/code/bsl-gradual-types/backend/src/bin/lsp_server/server/command_handlers.rs) сначала делает `spawn_bounded_blocking`, а уже внутри closure выбирает `ready_snapshot` либо `parser_coordinator`. Это означает, что current-context burst сначала конкурирует за background permits, а только потом узнаёт, что работа эквивалентна уже идущему parse.

## Goals
- Ограничить same-key current-context parse fan-out одним leader parse.
- Сохранить newest-generation-wins и fail-closed semantics.
- Избежать лишнего расхода blocking CPU permits на follower requests.
- Сделать результат наблюдаемым в incident bundles и perf gates.

## Non-Goals
- Не превращать `bsl.getCurrentContext` в interactive-priority путь.
- Не менять контракт client generation hints.
- Не делать глобальный parser singleflight rewrite для всех consumers.

## Decisions

### 1. Broker живёт над blocking boundary, а не внутри `ParserCoordinator`
Ключевое решение: новый broker должен срабатывать до входа в `spawn_bounded_blocking`, а не дополнять уже существующий `ParserCoordinator` singleflight.

Причина:

- проблема текущего инцидента не в duplicate parse как таковом, а в том, что followers уже успели взять scarce blocking capacity;
- `ParserCoordinator` решает duplicate parse body, но не решает duplicate blocking-holder problem;
- generation/supersession facts доступны именно на server/backend boundary, а не внутри generic parser service.

### 2. Contract: one leader, async followers, bounded empty on supersession/budget
Для эквивалентного current-context key:

- `ready_snapshot` завершает запрос сразу;
- если snapshot нет и leader отсутствует, создаётся один leader parse/context derivation;
- followers ждут shared result асинхронно через request-local future/watch/oneshot surface;
- superseded или over-budget follower MAY завершиться empty response, пока лидер продолжает прогрев reusable result.

Это сохраняет fail-closed semantics: stale context не возвращается newer generation, а полезный parse artifact всё ещё может быть переиспользован последующим актуальным запросом.

### 3. Budget остаётся bounded и fail-closed
`bsl.getCurrentContext` не обязан ждать десятки секунд ради auxiliary status surface. Поэтому broker должен иметь bounded wall decision:

- либо follower получает shared result вовремя;
- либо запрос возвращает empty response;
- либо superseded request завершается пустым ответом сразу.

Точное число миллисекунд допускается определить на реализации/validation этапе, но contract требует bounded empty outcome вместо unbounded follower hold.

### 4. Observability фиксирует не только parse source, но и broker role
Текущих parse-source metrics недостаточно: `parser_coordinator` не различает leader и follower wait. После change incident bundle должен уметь отвечать на вопросы:

- сколько current-context запросов были `ready_snapshot`;
- сколько реально запускали leader parse;
- сколько были broker followers;
- сколько были superseded/budget-exhausted.

Это позволит отделять “дорогой leader parse” от “много follower noise”.

## Alternatives Considered

### A. Оставить current path и только снизить CPU quota
Rejected.

Это уменьшит parallelism globally, но не уберёт сам duplicate holder pattern. Followers по-прежнему будут конкурировать за permits, только медленнее.

### B. Усилить `ParserCoordinator` singleflight внутри sync API
Rejected as primary fix.

Это не решает главный инцидентный symptom: follower уже вошёл в blocking boundary до того, как попал в singleflight wait.

### C. Перенести current-context parse на async runtime threads
Rejected.

Это нарушит уже существующий contract, что heavy auxiliary CPU work не должна выполняться inline на async runtime loops.

## Risks
- Если budget будет слишком агрессивным, статус-bar/current-context surface начнёт чаще возвращать empty result.
- Если broker key будет слишком узким, current-context bursts не будут coalesce-иться.
- Если broker key будет слишком широким, можно по ошибке слить несовместимые requests.

## Mitigations
- Валидировать key на same-file same-revision/text identity.
- Отдельно тестировать supersession и same-key burst behavior.
- Хранить ready-snapshot fast path выше broker, чтобы не penalize healthy exact-ready cases.
