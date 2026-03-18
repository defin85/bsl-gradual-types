## Context

После `refactor-v2-completion-dual-artifact-path` сервер уже умеет:
- выдавать current-revision `head_hit` response за единицы миллисекунд;
- различать `prepare_timeout` и `exact_deadline`;
- публиковать bounded per-request completion timeline через `bsl.getCompletionTimeline`.

Но текущая Observability panel по-прежнему отвечает только на server-side часть вопроса. На extension boundary нет привязанных к конкретному запросу сигналов о том:
- когда был последний локальный edit;
- какой `document.version` видел completion request;
- был ли completion quickly cancelled/superseded на клиенте;
- сколько прошло между локальной правкой и фактическим completion request.

Дополнительная проблема в том, что основной activation path использует `initializeLspClient -> buildClientOptions`, где сейчас нет completion middleware и нет client-side probe capture. Наличие coarse aggregate `PerformanceMonitor` не помогает: он усредняет операции, но не строит причинно-следственную цепочку для конкретного completion trace.

## Goals / Non-Goals

- Goals:
  - дать пользователю и разработчику per-request client-side контекст рядом с server timeline для цепочки `edit -> completion`;
  - сохранить server-driven timeline единственным источником истины для server stages, routes и outcomes;
  - сделать client probe feed bounded, локальным и безопасным по данным;
  - встроить instrumentation в default extension runtime path;
  - не создавать UI-level guesswork о том, какой local probe соответствует какому server trace.

- Non-Goals:
  - не вводить новый backend/LSP protocol surface в MVP;
  - не строить новую persistent telemetry pipeline;
  - не хранить raw document text, line prefixes или другие PII-like payloads;
  - не пытаться клиентом реконструировать server trace;
  - не делать trace-level correlation между local probes и server traces в рамках этого change.

## Architecture Drivers

- Debuggability: нужно быстро понимать, проблема начинается на client edge или уже внутри LSP/runtime.
- Safety: UI не должна создавать ложную причинность или подмешивать клиентские выводы в server truth.
- Boundedness: schema и retention должны оставаться low-cardinality и предсказуемыми.
- Scope control: MVP должен быть extension-only и не тянуть за собой новый backend contract.

## Alternatives Considered

### Option A: Оставить только server-driven timeline

Плюсы:
- нулевые изменения в extension instrumentation.

Минусы:
- остаётся blind spot между локальной правкой и server request;
- невозможно уверенно отделить client cancellation/edit churn от чисто server-side latency.

Вердикт: отклонено.

### Option B: Сразу добавить explicit `client_probe_id` в LSP/backend contract

Плюсы:
- идеальная корреляция request-to-request.

Минусы:
- меняет протокол и требует cross-stack rollout;
- увеличивает scope change и смешивает диагностический MVP с protocol evolution.

Вердикт: отложено как follow-up path, а не как часть этого change.

### Option C: Generic telemetry logger / persistent diagnostics log

Плюсы:
- можно собирать больше данных для долгих сессий.

Минусы:
- слишком тяжёлый операционный след для текущей задачи;
- выше риск data drift и privacy noise;
- не нужен для first diagnostic MVP.

Вердикт: отклонено.

### Option D: Local in-memory completion probes + independent client probe feed

Плюсы:
- покрывает текущий observability gap без backend changes;
- остаётся локальным, bounded и дешёвым;
- позволяет быстро увидеть `time_since_last_edit`, client cancellation и версию документа рядом с server timeline;
- не создаёт trace-level guesswork и не требует локальной эвристики.

Минусы:
- не даёт автоматического ответа, какой local probe соответствует какому server trace;
- ручной анализ требует чуть больше когнитивной работы.

Вердикт: рекомендуется для MVP.

## Decisions

### Decision: Server trace остаётся authoritative, client probe feed только local debug stream

Observability completion UI продолжает читать server trace только из `bsl.getCompletionTimeline`.
`Client Probe Feed` может отображаться рядом, но:
- не заменяет server outcome/route/stage;
- не заполняет пропущенные server stages;
- не влияет на server contract versioning;
- не трактуется как surrogate authoritative timeline.

### Decision: Probe capture живёт в основном VS Code client path

Instrumentation MUST быть подключена к реально используемому `buildClientOptions`/`initializeLspClient` path. Optional `enhanced-client` может использовать те же helper-ы, но не считается достаточным wiring evidence.

### Decision: Probe schema bounded и redacted

MVP probe хранит только:
- `probe_id`;
- `uri`;
- `document_version`;
- `trigger_mode` и optional `trigger_character`;
- `request_started_at_ms` и terminal timestamp/result;
- `time_since_last_local_edit_ms`;
- `time_since_last_did_change_sent_ms` или явный `unknown`, если сигнал недоступен в конкретном path;
- derived context flags вроде `is_after_dot` и `identifier_tail_length`.

MVP probe НЕ хранит:
- raw line/document text;
- unbounded snippets;
- high-cardinality free-form labels.

### Decision: MVP не делает trace-level correlation вообще

В текущем change UI НЕ пытается сопоставлять local probe с конкретным server trace.

Причины:
- даже детерминированная локальная эвристика остаётся guesswork;
- UI-level guesswork создаёт риск “правдоподобной, но не explainable” причинности;
- отсутствие correlation упрощает contract, тесты и user interpretation.

Следствие:
- `Server Timeline` остаётся самостоятельной authoritative surface;
- `Client Probe Feed` остаётся самостоятельной local-only debug surface;
- пользователь сопоставляет их вручную по времени, `uri`, `document_version` и trigger metadata.

### Decision: Exact correlation через `client_probe_id` остаётся отдельным follow-up

Если после внедрения dual-view MVP потребуется machine-join между local probes и server traces, следующим архитектурным шагом должен быть отдельный change с protocol-assisted correlation:
- extension создаёт `client_probe_id`;
- correlation id передаётся в LSP по отдельному observability/debug surface;
- `bsl.getCompletionTimeline` возвращает optional `client_probe_id`;
- UI выполняет exact join по id вместо локальной догадки.

Этот protocol-assisted path явно не входит в scope текущего change.

## Proposed Architecture

### Data flow

1. Extension слушает локальные текстовые изменения и обновляет lightweight edit clock по `uri`.
2. Completion middleware на основном `LanguageClient` path создаёт probe перед `next(...)`.
3. После completion result/cancellation probe завершается и кладётся в bounded in-memory ring buffer.
4. `Server Timeline` продолжает получать server traces через `bsl.getCompletionTimeline`.
5. Model layer формирует два независимых snapshot-а: `Server Timeline` и `Client Probe Feed`.

### UI model

Observability completion UI в MVP отображает два независимых блока:
- `Server Timeline`: server summary и stage timeline как сегодня;
- `Client Probe Feed`: список последних local probes без join к конкретным server traces.

Каждая запись в `Client Probe Feed` должна включать только bounded поля:
- `document_version`;
- trigger metadata;
- `time_since_last_local_edit_ms`;
- `time_since_last_did_change_sent_ms`/`unknown`;
- `client_duration_ms`;
- `client_terminal_state`.

### Retention

- Ring buffer count-based, bounded, oldest-first eviction.
- Buffer хранится только в памяти extension session.
- Clipboard/export формат обязан явно отделять `Server Timeline` от `Client Probe Feed`.

### Legacy / unsupported behavior

Если подключённый LSP не поддерживает `bsl.getCompletionTimeline`:
- `Server Timeline` section MUST показывать явный unsupported status;
- `Client Probe Feed` MAY оставаться доступным как local-only debug stream;
- UI MUST явно сообщать, что local probes не эквивалентны authoritative server timeline.

## Validation Strategy

- Unit tests:
  - deterministic eviction;
  - отсутствие raw text в probe payload;
  - корректный independent rendering server/client streams;
  - отсутствие trace-level correlation и синтеза server stages из client-side данных.
- Extension integration tests:
  - probe capture wired on default client path;
  - `Server Timeline` и `Client Probe Feed` рендерятся независимо друг от друга;
  - при unsupported `bsl.getCompletionTimeline` server section fail-closed, а local probe section не маскирует отсутствие authoritative timeline.
- Tooling:
  - `npm run lint` в `vscode-extension/`;
  - `openspec validate add-extension-completion-probe-monitoring --strict --no-interactive`.

## Risks / Trade-offs

- Отсутствие automatic correlation означает более высокую когнитивную нагрузку при ручном анализе.
- Dual-view MVP решает trustworthiness, но не даёт автоматического ответа “этот probe соответствует именно этому trace”.
- Даже после добавления probes останется backend-side latency work; этот change улучшает diagnosis, а не сам completion throughput.
- Если UI недостаточно чётко разделит server trace и local probe feed, пользователи могут принять клиентские сигналы за server truth.

## References

- VS Code telemetry guide: https://code.visualstudio.com/api/extension-guides/telemetry
- VS Code API reference: https://code.visualstudio.com/api/references/vscode-api
- `vscode-languageclient` middleware types: `vscode-extension/node_modules/vscode-languageclient/lib/common/completion.d.ts`
- `vscode-languageclient` text synchronization middleware types: `vscode-extension/node_modules/vscode-languageclient/lib/common/textSynchronization.d.ts`
