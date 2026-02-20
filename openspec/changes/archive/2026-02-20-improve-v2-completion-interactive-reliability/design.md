## Context

Текущий v2 completion pipeline уже поддерживает stale/incomplete fallback и trigger characters в server capabilities, но в интерактивном редактировании остаются UX-проблемы:
- completion может заметно задерживаться после серии `didChange`;
- первый completion в строке может вернуться "пустым" и требовать повторного ручного вызова;
- completion по `.` не всегда даёт ожидаемый результат в той же ревизии, где `Ctrl+Space` даёт.

Наблюдаемое в коде:
- hot path `didChange` ждёт применение предыдущей версии перед склейкой incremental edits;
- при временной недоступности IR completion может уйти в empty-response ветку;
- completion не использует `CompletionParams.context` для trigger-aware политики;
- owner-hint для member-access завязан на наличие IR и теряется при transient-cancel.

## Goals / Non-Goals

- Goals:
  - Стабилизировать интерактивный completion UX: полезный первый ответ без ручного retrigger.
  - Обеспечить semantic parity между trigger по `.` и ручным `Ctrl+Space`.
  - Ограничить latency completion под typing-load и зафиксировать контракт деградации.
  - Сделать причину "по точке нет подсказок" диагностируемой на стороне extension.
- Non-Goals:
  - Переработка ranking/score модели completion.
  - Расширение доменной coverage типов/metadata beyond текущего контракта.
  - Изменение пользовательских editor settings автоматически.
  - Полная event-driven rearchitecture очередей runtime/LSP (выделена в отдельный change `refactor-v2-completion-event-driven-pipeline`).

## Decisions

### Decision 1: LSP adapter хранит согласованный latest document shadow state

Для интерактивных операций completion source-of-truth по тексту/версии документа должен быть доступен сразу после приёма `didChange`, без принудительной блокировки на предыдущую версию в hot path.

Это снижает окно гонки `didChange -> completion` и убирает класс "первый completion пустой из-за transient lag".

### Decision 2: Completion становится trigger-aware по LSP context

Server completion policy должна явно учитывать `CompletionParams.context`:
- `TriggerCharacter='.'` не должен деградировать в keyword-only/irrelevant fallback для member-access контекста;
- `Invoked` и `TriggerCharacter` на одной ревизии должны давать согласованную semantic выдачу (допуская `isIncomplete=true` в деградированном пути).

### Decision 3: Деградированный путь должен быть полезным, а не terminal-empty

Если fresh semantic snapshot временно недоступен, completion должен возвращать полезный degraded результат (non-empty candidates при распознаваемом member-access контексте) и маркировать его `isIncomplete=true`.

`Terminal empty` допустим только когда контекст реально не даёт кандидатов, а не как следствие transient IR unavailability.

### Decision 4: Owner hint для `expr.` вычисляется независимо от strict IR availability

Member-owner hint не должен полностью зависеть от готовности IR в момент запроса. Нужно поддержать fallback-путь, который сохраняет корректный receiver-context для `expr.` при transient-cancel и тем самым избегает ложной деградации.

### Decision 5: Extension диагностирует выключенные completion trigger settings

Расширение не должно молча оставлять пользователя в состоянии "по точке ничего не происходит", если effective editor settings отключают trigger-based suggestions.

Контракт: при старте/активации extension явно логирует warning и даёт понятный remediation path, не изменяя пользовательские настройки автоматически.

### Decision 6: Полная event-driven rearchitecture выделяется в отдельный трек

Для этого change выбирается эволюционный путь (runtime-centric contract + adapter shadow state) с минимальным риском регрессий.

Полная event-driven rearchitecture очередей и диспетчеризации completion/didChange вынесена в отдельный change `refactor-v2-completion-event-driven-pipeline`, чтобы не блокировать быстрые UX-исправления текущего цикла.

### Decision 7: Observability фиксируется с trigger-aware разрезом

Для контроля parity и деградации observability должна включать как минимум:
- trigger mode (`TriggerCharacter`, `Invoked`, `TriggerForIncompleteCompletions`),
- completion outcome class (`ok_non_empty`, `ok_empty`, `degraded_incomplete`, `fallback_unavailable`),
- parity drift indicator между `.` и `Invoked` в рамках одной ревизии.

Это нужно, чтобы регрессии parity обнаруживались по метрикам, а не только тестами.

## Implementation Considerations

1. Ввести/расширить LSP-level document shadow state для `didOpen/didChange/didClose`.
2. Рефакторинг completion hot path под trigger-aware ветвление и unified fallback policy.
3. Устранить ветки, которые дают terminal-empty из-за transient `missing_ir` в member-access контексте.
4. Добавить parity/first-trigger/latency regression tests в LSP integration набор.
5. Добавить extension-level observable warning (output channel + user-visible lightweight notification/diagnostic hook).
6. Добавить trigger-aware observability (разрез по trigger mode, parity drift, terminal-empty в member-access контексте).

## Acceptance SLI (для этого change)

- Warm-path completion latency под профильной нагрузкой:
  - `p95 <= 300ms`
  - `p99 <= 800ms`
- First-trigger success rate для member-access после `didChange`: `>= 99%`.
- Terminal-empty rate в распознанном member-access контексте из-за transient `missing_ir`: `<= 0.5%`.
- Parity mismatch между `TriggerCharacter='.'` и `Invoked`:
  - `0` в контрактных тестах,
  - `<= 1%` на нагрузочном telemetry-срезе.

## Risks / Trade-offs

- Риск "слишком агрессивного" degraded fallback (шум/лишние candidates).
  - Митигация: ограничивать degraded путь member-access контекстом и помечать `isIncomplete=true`.
- Риск, что stricter parity между trigger modes повысит стоимость вычислений.
  - Митигация: reuse snapshot/query results и bounded budgets.
- Риск неполной диагностики клиентских настроек в multi-root/workspace override сценариях.
  - Митигация: проверять effective configuration для активного BSL документа и логировать источник значения.

## Open Questions

- Нужен ли отдельный пользовательский indicator для degraded completion (кроме `isIncomplete=true`) в UI extension.
