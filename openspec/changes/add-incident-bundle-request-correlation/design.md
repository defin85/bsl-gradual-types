## Контекст
Сейчас bundle export уже сохраняет:
- authoritative `raw/completion_timeline.json`;
- local-only `raw/client_probes.json`;
- cumulative `raw/observability_metrics.json`;
- краткий `summary.md`;
- machine-readable `incident.json`.

Практический разбор последних bundle показал два класса проблем:
1. Raw-вложения достаточно информативны, чтобы локализовать проблему.
2. Derived report почти не помогает автоматическому анализу, потому что не содержит request-centric структуры.

Типичный симптом:
- все traces относятся к одному `uri`, но `incident.json` это не выражает;
- по summary видно, что есть `prepare_timeout` или `exact_deadline`, но не видно, какие именно requests были затронуты и какими bounded facts это подтверждается;
- client probe timestamps уже есть, но derived report не использует их даже для conservative correlation.

## Цели
- Сделать `incident.json` пригодным для AI/incident handoff без обязательного чтения raw JSON.
- Сохранить raw attachments как source of truth.
- Использовать только уже существующие extension-side и server-side данные.
- Делать correlation только там, где она детерминирована и bounded.

## Не-цели
- Не менять LSP contract и не добавлять новый серверный transport.
- Не строить псевдо-authoritative server trace из client probes.
- Не вычислять `metrics delta` из одного snapshot.
- Не добавлять новый UI panel; результат остаётся внутри bundle export.

## Решение

### 1. Capture scope и request window
Derived report должен явно выражать scope текущего bundle:
- `uri`, если все authoritative traces относятся к одному документу;
- иначе bounded summary по количеству `uri` без выбора "главного" документа guesswork-ом;
- `request_count` по authoritative completion traces;
- `request_window` с явной политикой, что основная request-centric truth строится от server timeline, а не от probes.

Если authoritative timeline недоступен:
- bundle остаётся валидным partial export;
- request-centric section помечается как unavailable/unsupported;
- local probes не подменяют missing server request list.

### 2. Request-centric derived summary
`incident.json` должен включать bounded список requests, где каждая запись строится поверх одного authoritative server trace и содержит только low-cardinality/числовые поля:
- `trace_id`, `request_id`, `uri`, `trigger_mode`, `outcome`;
- `total_duration_ms`, `dominant_stage`;
- server-edge latencies:
  - `transport_to_handler_wait_ms`;
  - optional `transport_to_method_wait_ms`;
  - optional `method_prelude_exec_ms`;
  - `server_handler_exec_ms`;
- bounded bottleneck verdict list;
- optional prepare/exact fact block, если trace содержит `prepare_timeout` или `exact_deadline`.

Это не заменяет raw trace, а даёт компактный индекс по проблемным requests.

### 3. Conservative client/server correlation
Optional correlation строится только при достаточных данных:
- authoritative trace доступен;
- candidate probe имеет тот же `uri`;
- сопоставление deterministic по bounded ordering/timestamp rules;
- нет неоднозначности между несколькими probe-кандидатами.

Если correlation уверенная, request summary MAY включать:
- `probe_id`;
- `client_duration_ms`;
- `client_terminal_state`;
- optional client/server edge deltas вроде `client_to_transport_wait_ms`.

Если correlation неуверенная:
- request остаётся только server-centric;
- derived report явно фиксирует gap или `correlation_status=unavailable/ambiguous`;
- extension не выдумывает пары `probe <-> trace`.

### 4. Summary markdown
`summary.md` должен получить короткий request-centric section:
- `uri`/scope;
- request count;
- список 3-5 key requests или compact request bullets;
- явную заметку, если client/server correlation unavailable или ambiguous.

## Архитектурные решения
- Source of truth для request list: authoritative completion timeline.
- Source of truth для client-side supplement: local probe buffer.
- Source of truth для metrics: cumulative metrics snapshot без delta.
- Derived report остаётся additive слоем поверх raw attachments.

## Риски и trade-offs

### Риск: ложное probe/trace сопоставление
Смягчение:
- correlation только при строгих deterministic правилах;
- ambiguous case превращается в explicit gap, а не в guessed pair.

### Риск: разрастание `incident.json`
Смягчение:
- request entries bounded по `completion_trace_limit`;
- внутри request summary только ключевые числовые и bounded vocabulary поля;
- без raw stage arrays и без дублирования полного timeline payload.

### Риск: дублирование raw и derived semantics
Смягчение:
- raw attachments остаются обязательными;
- derived report не копирует полный trace целиком;
- summary формулируется как index/handoff, а не как replacement raw evidence.

## Acceptance-направление
- Single-document capture window явно выражает `uri` и `request_count`.
- `incident.json` содержит request list для authoritative traces.
- `prepare_timeout` и `exact_deadline` читаются из request summary без открытия raw JSON.
- При ambiguous correlation bundle остаётся валидным и явно помечает ограничение.
- Metrics snapshot остаётся cumulative section без псевдо-`delta`.
