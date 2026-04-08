## Контекст

Phase 1 отделяет:

- `response_ready_to_output_enqueue_wait_ms`;
- `response_output_queue_wait_ms`;
- `response_output_encode_exec_ms`;
- `response_output_write_and_flush_exec_ms`.

Но сама причина большого `response_output_queue_wait_ms` после этого всё ещё остаётся opaque без truthful ahead snapshot.

Текущий output loop merge-ит разные источники outbound traffic и не хранит достаточной metadata для authoritative head blocker attribution.

## Versioning Note

Текущий target-state после phase 1:

- `response.version = 22`;
- `contracts/lsp-completion-timeline/v19`.

Эта фаза строится поверх него и целится в:

- `response.version = 23`;
- `contracts/lsp-completion-timeline/v20`.

## Goals

- Объяснить `response_output_queue_wait_ms` через bounded ahead snapshot.
- Ввести truthful coarse blocker attribution для completion response в output path.
- Сохранить additive/backward-compatible semantics для уже shipped `v22` полей.

## Non-Goals

- Не менять ordering/fairness policy output path.
- Не лечить сам output backlog в рамках этого change.
- Не публиковать exact flushed byte accounting или raw queue dump.

## Решения

### 1. Unified outbound envelope path становится authoritative truth source для backlog attribution

Оба источника outbound traffic должны проходить через единый bounded envelope path перед writer.

Envelope обязан нести:

- исходное сообщение;
- coarse `origin_class`;
- optional `completion_request_id` для completion-correlated response;
- bounded size estimate metadata, достаточную для `output_bytes_ahead_estimate`.

Без этого `output_head_blocker_class` и `output_messages_ahead_count` останутся guessed.

### 2. Snapshot semantics фиксируются на enqueue boundary и включают active writer head

Для completion envelope ahead snapshot фиксируется в момент успешного enqueue completion.

Snapshot MUST включать:

- уже пишущийся active head, если writer занят;
- queued envelopes, которые гарантированно стоят впереди completion envelope по FIFO path.

Это даёт truthful explanation для последующего `response_output_queue_wait_ms` без post-hoc reconstruction.

### 3. Bytes field остаётся estimate, а не exact flushed byte count

`output_bytes_ahead_estimate` должен отражать bounded best-effort оценку header+body bytes для ahead envelopes.

Consumers:

- MUST NOT трактовать его как exact flushed byte count;
- MAY использовать его только как coarse indicator масштаба backlog.

### 4. `v22` degradation остаётся fail-closed

На `v22` surfaces:

- не выдумывают `output_messages_ahead_count`, `output_bytes_ahead_estimate` или `output_head_blocker_class`;
- прямо говорят, что truthful backlog attribution unavailable by design;
- не переименовывают `response_output_queue_wait_ms` в `notification backlog` или иной конкретный culprit.

## Alternatives Considered

### Пытаться вычислять backlog snapshot на текущем merged output loop без envelope refactor

Отклонено. Это приводит к guessed blocker attribution и неполному учёту active head.

### Публиковать exact `output_bytes_ahead`

Отклонено. Exact field стимулирует pre-encode и рискует исказить timing seam, который change расследует.

## Риски и Trade-offs

### Риск: unified envelope path затронет generic transport adapter

Смягчение:

- ограничить новый metadata-carrier bounded coarse fields;
- не менять admission/fairness policy, только observability ownership у writer path.

### Риск: snapshot estimate будет noisy

Смягчение:

- использовать имя `output_bytes_ahead_estimate`;
- явно документировать best-effort semantics и bounded vocabulary.

### Риск: surfaces перегрузятся

Смягчение:

- показывать snapshot только при `response.version >= 23`;
- оставить queue wait как primary bucket, а ahead snapshot как supporting evidence.
