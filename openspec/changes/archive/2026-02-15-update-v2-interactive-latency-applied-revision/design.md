## Context
Warm-path метрики показывают структурную проблему интерактивного пайплайна:
- высокий cancel-rate completion;
- tail latency в `wait_for_file_version`/`ir_query` на уровне секунд;
- UX-паттерн "первый completion пустой, повторный — осмысленный".

Текущее состояние:
- Singleflight дедуплицирует expensive query только внутри точного revision key, включая `file_version`.
- `prepare_lsp_stateful_operation_v2` ориентируется на latest received version.
- CPU budget имеет симметричный borrow, который в пиковых состояниях не защищает интерактивный путь достаточно жёстко.

## Goals / Non-Goals
- Goals:
  - Снизить tail latency интерактивных операций на warm-path.
  - Убрать "пустой первый completion" при допустимом stale snapshot.
  - Сохранить strict latest-version публикацию diagnostics.
  - Сделать причины деградации прозрачными через observability.
- Non-Goals:
  - Полный redesign анализа/типизации.
  - Изменение LSP API контракта beyond стандартные completion semantics.

## Decisions

### Decision 1: Разделить `received_version` и `applied_version`
- Ввести явный runtime контракт: интерактивная готовность определяется по `applied_version`.
- `received_version` остаётся входной ревизией transport-уровня, но не является доказательством готовности semantic snapshot.

Rationale:
- Снимает ложную готовность, когда didChange уже получен, но runtime ещё не применил SetFile.

### Decision 2: Приоритизировать control-path относительно query-path
- Control-path команды (apply changes, wait-for-version coordination) должны исполняться с более высоким приоритетом.
- Background-путь не должен отбирать интерактивную гарантию при наличии interactive waiters.

Rationale:
- Уменьшает очереди перед интерактивными запросами и предотвращает cascade latency.

### Decision 3: Completion stale-first fallback при bounded timeout/cancel
- Для completion при недоступности latest в пределах wait budget система возвращает stale-compatible результат (если проходит stale limits + deps/settings match).
- Ответ помечается как частичный (`isIncomplete=true`) и дозапрашивается повторно стандартным поведением клиента.
- Если безопасного stale snapshot нет, completion возвращает быстрый empty/partial без длительной блокировки.

Rationale:
- Для UX completion полезнее быстрый частичный ответ, чем пустой результат после долгого ожидания.

### Decision 4: Расширить observability на lag/fallback причины
- Добавить фиксированные метрики lag между received/applied и fallback outcomes.
- Добавить quality-gate по cancel-rate completion, а не только по latency.

Rationale:
- Нужна диагностируемость: видеть, что проблема в lag/queue contention, а не в repository/deps reload.

## Alternatives Considered
- Увеличить wait budget для интерактива:
  - Отвергнуто: снижает частоту timeout, но ухудшает UX и не устраняет root cause очередей.
- Убрать `file_version` из singleflight key:
  - Отвергнуто: риск semantically stale sharing между разными ревизиями.
- Агрессивно кэшировать готовые completion-выдачи:
  - Отложено: выше риск рассинхронизации и сложнее invalidate-policy.

## Risks / Trade-offs
- Риск устаревшего completion при stale fallback:
  - Mitigation: строгая проверка `deps_id/settings_id`, stale limits и `isIncomplete=true`.
- Риск starvation background при сильном интерактивном приоритете:
  - Mitigation: минимум 1 background permit и fairness-квота.
- Риск усложнения runtime coordination:
  - Mitigation: минимальный scope (только revision tracking + lane priority + metrics), regression/perf tests.

## Migration Plan
1. Специфицировать dual-revision модель и latency contract.
2. Внедрить applied-revision tracking и control-path priority.
3. Внедрить completion stale-first fallback.
4. Добавить/обновить метрики и perf gates.
5. Прогнать parity/perf smoke и сравнить cold/warm профиль.

## Open Questions
- Нужен ли отдельный runtime knob для strict-запрета stale fallback в completion для debugging/CI?
- Нужен ли hard cap на число подряд stale completion ответов на один файл до принудительного latest-only запроса?
