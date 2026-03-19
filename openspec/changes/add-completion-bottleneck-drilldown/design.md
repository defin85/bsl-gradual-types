## Контекст
После добавления per-request completion timeline и incident bundle стало возможно локализовать часть completion-проблем без отдельного профилирования. Практический разбор показал, что основные bottleneck'и в реальных инцидентах распадаются как минимум на три класса:

- задержка до входа в completion handler (`transport_to_handler_wait_ms` при почти нулевом `turn_wait`);
- `prepare_timeout`, который сейчас виден по terminal outcome, но недостаточно ясно показывает subphase/runtime split;
- `exact_deadline` после успешного `prepare`, когда требуется понять состояние waiter/task, а не только сам факт deadline.

Одновременно выяснилось, что часть уже существующего bounded drilldown остаётся raw-only:

- `turn_attribution.dispatcher_resolution_latency_ms`;
- `prepare_details.progress`;
- `prepare_details.exact_wait`.

Именно поэтому текущий handoff поток всё ещё требует ручного чтения raw JSON, хотя authoritative structured data уже частично существует.

## Goals / Non-Goals

### Goals
- Сделать типовые completion bottleneck'и локализуемыми из одного authoritative payload без ad-hoc логов.
- Сохранить low-cardinality, bounded и versioned contract для `bsl.getCompletionTimeline`.
- Сделать новые drilldown-поля видимыми в человекочитаемых проекциях: panel, clipboard, incident handoff summary.
- Сохранить graceful degradation для старых payload и fail-open для instrumentation.

### Non-Goals
- Не вводить отдельный постоянный log file или NDJSON pipeline.
- Не строить новый observability API поверх timeline.
- Не превращать incident summary в замену raw attachments.
- Не пытаться в этом change решать сами performance-проблемы completion; change только улучшает локализацию root cause.

## Решения

### 1. Авторитетной поверхностью остаётся `bsl.getCompletionTimeline`
Новый drilldown остаётся частью versioned per-request completion contract. Мы не добавляем отдельный экспорт из backend и не переносим диагностику в text logs.

Причины:
- existing transport уже есть в extension и incident bundle;
- raw timeline лучше коррелируется с конкретным completion request, чем cumulative metrics или текстовые логи;
- новая поверхность увеличила бы риск semantic drift между несколькими diagnostic source of truth.

Следствие:
- change должен bump'нуть contract version;
- extension должен уметь читать старый payload и явно деградировать при отсутствии новых полей.

### 2. Drilldown остаётся bounded и low-cardinality
Новые поля должны описывать состояние fixed vocabulary, а не свободный текст:

- ingress/disptacher attribution через числовые latency и bounded outcome/status поля;
- prepare drilldown через bounded subphase/runtime breakdown;
- exact-wait drilldown через bounded waiter/task state.

Недопустимы:
- URI/пути/имена символов как часть новых labels;
- stack traces;
- adapter-local free-text причины, которые нельзя стабильно тестировать.

### 3. Prepare drilldown должен отделять subphase от runtime split
Для `prepare_stateful` недостаточно terminal `prepare_timeout`. Требуется увидеть:

- в какой subphase остановился prepare;
- произошёл ли bottleneck в `wait_for_file_version` или в `snapshot_with_deps`;
- если данные доступны, как раскладывается runtime path на queue-wait vs execute/wake path.

Минимальная полезная модель:
- сохранить текущий `progress` как coarse phase marker;
- дополнить `prepare_details` bounded runtime objects для `wait_for_file_version` и `snapshot_with_deps`;
- при частичной недоступности этих деталей оставлять поля `undefined`, а не синтезировать значения.

### 4. Exact wait должен объяснять не только deadline, но и состояние precompute
Текущий факт `exact_deadline` недостаточен. Для root-cause анализа нужны bounded ответы на вопросы:

- был ли matching precompute task;
- join/promotion произошёл или нет;
- в какой phase находился task;
- совпадала ли ожидаемая версия.

Новые поля должны дополнять, а не заменять существующий `exact_wait` блок.

### 5. Human-readable projections не должны прятать authoritative drilldown
Новые/уже существующие bounded поля должны появиться не только в raw JSON, но и в:

- Completion Timeline panel;
- clipboard export;
- incident handoff summary (`summary.md` / `incident.json`).

Принцип:
- raw attachments остаются source of truth;
- derived projection обязана передавать типовой bottleneck verdict без ручного парсинга raw JSON;
- summary не должен придумывать данные, если конкретное bounded поле отсутствует.

## Предлагаемая модель данных

### Timeline contract `v5`
Контракт расширяется как additive superset текущего payload:

- `turn_attribution.dispatcher_resolution_latency_ms` остаётся bounded ingress attribution и становится обязательной частью human-readable projection, когда присутствует;
- `prepare_details.progress` сохраняется как coarse phase marker;
- `prepare_details` получает bounded runtime drilldown для:
  - `wait_for_file_version`;
  - `snapshot_with_deps`;
- `prepare_details.exact_wait` дополняется bounded waiter/task-state полями.

Точные названия полей будут закреплены в spec delta; design фиксирует только semantic intent:
- latency/value fields — numeric milliseconds или versions;
- state fields — только из fixed vocabulary.

### Human-readable verdicts
Для типовых случаев projection должен уметь выразить:

- `ingress_dominant`: `transport_to_handler_wait_ms` существенно больше handler work, при этом dispatcher latency либо мала, либо отдельно показана;
- `prepare_timeout@wait_for_file_version` или `prepare_timeout@snapshot_with_deps`;
- `exact_deadline` с пояснением waiter/task state;
- `head_hit`/healthy hot path без лишнего шума.

## Совместимость и rollout
- Backend bump'ает timeline contract version.
- Extension принимает `v4` и `v5`:
  - для `v4` показывает старые данные и явно помечает недоступность нового drilldown;
  - для `v5` включает расширенную human-readable projection.
- Incident bundle не должен ломаться на старом payload: он остаётся частичным, но явным.

## Риски / Trade-offs
- Чем больше полей в timeline, тем выше риск контрактного шума. Смягчение: только bounded state + numeric fields, без свободного текста.
- Добавление runtime split в request path может само стать источником overhead. Смягчение: только дешёвые timestamp/state capture, без sync IO.
- Extension может начать дублировать server semantics своей эвристикой. Смягчение: derived verdict строится только из structured authoritative fields и не invent'ит значения.

## Validation Strategy
- Backend contract tests должны проверять:
  - version bump;
  - bounded vocabulary;
  - additive/fail-open semantics при частично отсутствующих деталях;
  - отсутствие semantic drift между terminal outcome и новым drilldown.
- Extension tests должны проверять:
  - panel/clipboard отображают новые bounded поля;
  - incident summary умеет формулировать типовой bottleneck verdict;
  - старый `v4` payload деградирует явно, без краша и без invented data.
