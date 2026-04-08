## Context

`add-04-diagnostics-save-timeline-incident-bundle` уже добавил authoritative `didSave` trace в bundle, но live
capture выявил два недочёта runtime lifecycle:

- terminal cycle архивируется, а поздний `idle_heavy` result для того же key снова создаёт partial active trace;
- `save_fastlane` first publish может spend секунды до actual syntax query, но trace фиксирует только
  `syntax_diagnostics_query_ms`, из-за чего root cause снова partially hidden.

В UI-summary это усугубляется тем, что active cycles рендерятся как `unknown`, хотя реальное состояние уже известно:
refresh ещё идёт.

## Goals

- Один save-cycle должен иметь ровно один authoritative trace identity.
- Terminal cycle должен быть immutable; late completions можно только игнорировать, но не resurrect.
- `save_fastlane` first publish должен отделять blocking queue wait от actual syntax query work.
- Summary должен явно различать `in_flight` и terminal traces.

## Non-Goals

- Не строить отдельный per-`didChange` request trace.
- Не вводить unbounded history для terminal keys.
- Не менять existing completion timeline contract.

## Decisions

### 1. Terminal tombstones for save-cycle keys

После terminal archive сервер будет сохранять bounded set recently terminal
`DiagnosticsSaveTimelineCycleKey`.

`record_diagnostics_save_timeline_profile_result(...)` и convenience-paths MUST:

- сначала проверять, не был ли key уже terminal;
- игнорировать поздний result для tombstoned key;
- не создавать новый trace через `or_insert_with(...)`, если key уже terminal.

Это даёт idempotent lifecycle без переписывания archived traces задним числом.

### 2. Explicit fastlane queue-wait attribution

`save_fastlane` shadow-parse fallback должен использовать observed blocking call variant, чтобы trace получил
optional `blocking_queue_wait_ms` для first/follow-up publish facts.

Это поле дополняет existing stage facts:

- не заменяет `syntax_diagnostics_query_ms`;
- не требует raw runtime queue snapshot;
- заполняется только когда профиль действительно проходил через bounded blocking queue.

### 3. In-flight summary rendering

Extension summary не меняет server truth, но проецирует active traces fail-closed:

- если `terminal_outcome` отсутствует, summary рендерит `terminal=in_flight`;
- missing profile outcome при active cycle рендерится как `pending`, а не `unknown`.

Это уменьшает шум в incident bundle и не маскирует реальное состояние.

## Risks / Trade-offs

- Tombstone retention слишком маленького размера может допустить resurrection очень старого late result. Бounded
  retention должен совпадать с trace retention, этого достаточно для live incident workflow.
- Добавление optional field в trace response требует contract version bump и совместного обновления backend/extension.
- Queue-wait attribution будет truthfully empty для applied-analysis и ready-parse fastlane path; это нормально, потому
  что проблема относится только к blocking fallback path.

## Validation

- regression: duplicate trace не появляется после terminal archive и позднего superseded result;
- regression: first publish trace содержит `blocking_queue_wait_ms`, когда parse fallback искусственно стоит в очереди;
- bundle summary tests: active cycle рендерится как `in_flight/pending`, а не `unknown`.
