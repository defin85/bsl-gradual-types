## Контекст
`add-extension-completion-probe-monitoring` уже добавил dual-view observability: authoritative `Server Timeline` и local-only `Client Probe Feed`. Это решило проблему доверия к данным, но current probe schema всё ещё слишком бедна для разбора двух реальных классов проблем:
- auto-cancelled completion probes непонятно почему отменились;
- long `ok_empty` probes непонятно, где именно проводят время.

При этом серверный contract `bsl.getCompletionTimeline` уже стабилизирован, а предыдущий change сознательно запретил trace-level correlation между server traces и local probes.

## Goals / Non-Goals

### Goals
- Расширить client-side probes bounded/redacted diagnostics, не меняя server contract.
- Сделать auto-cancel / supersede path объяснимым на стороне extension.
- Сделать long-empty / slow probe path разложимым на transport/client phases.
- Добавить enough-local-context для version drift и overlap without raw text.

### Non-Goals
- Не добавлять protocol-level exact correlation (`client_probe_id`) в этом change.
- Не менять shape/version `bsl.getCompletionTimeline`.
- Не строить общую причинно-следственную timeline из server traces и local probes.
- Не добавлять persistent telemetry/export pipeline.

## Решения

### Decision: Enriched diagnostics остаются только в local probe schema
Новые поля живут только в extension-local probe payload и отображаются только в `Client Probe Feed`.

Следствие:
- `Server Timeline` остаётся authoritative representation server-generated payload;
- `Client Probe Feed` остаётся отдельным local-only debug stream;
- backend/LSP protocol, contracts и observability version не меняются.

### Decision: Cancellation diagnosis остаётся bounded and heuristic
Для отменённых probe добавляется bounded `cancel_reason_hint` со словарём:
- `superseded_same_version`
- `superseded_newer_version`
- `editor_state_changed`
- `unknown`

Дополнительно probe MAY содержать:
- `superseded_by_probe_id`
- `superseded_after_ms`

Эти поля выводятся только из extension-local sequencing/state и не претендуют на exact join с server trace.

### Decision: Transport-phase timing uses explicit local milestones
Для разбора `client_duration_ms` probe MUST иметь enough explicit timestamps to separate:
- client enter/start;
- LSP request dispatch;
- LSP response receive;
- client terminal/render-ready completion.

Чтобы избежать ненужного breaking churn в UI/clipboard, существующие `request_started_at_ms` и `request_completed_at_ms` сохраняются, а дополнительные transport milestones добавляются рядом.

### Decision: Result-shape и overlap metadata остаются bounded
Result-shape diagnostics используют bounded vocabulary:
- `result_kind`: `non_empty|empty_array|empty_list|nullish`
- `item_count_bucket`: `0|1_5|6_20|21_plus`
- `is_incomplete`: только если этот сигнал доступен без guesswork

Overlap/version-drift diagnostics ограничиваются bounded counters/flags:
- `document_version_at_terminal`
- `did_change_count_during_probe`
- `cursor_moved_during_probe`
- `active_completion_count_at_start`
- `same_uri_probe_overlap_count`
- `newer_probe_started_before_terminal`

Никаких raw snippets, line prefixes или unbounded labels поверх уже shipped probe schema не добавляется.

## Альтернативы

### Option A: Оставить только текущие probes
Плюсы:
- нулевой implementation cost.

Минусы:
- не объясняет long `ok_empty`;
- не объясняет auto-cancel without explicit user cancellation;
- оставляет слепую зону между client middleware и server timeline.

Отклонено.

### Option B: Сразу добавить protocol-level `client_probe_id`
Плюсы:
- exact machine correlation между client probe и server trace.

Минусы:
- требует cross-stack change в extension, LSP, contract и acceptance flow;
- расширяет scope сильнее, чем нужно для ближайшей diagnosis задачи.

Отклонено для этого change, оставлено follow-up option.

## Риски / Trade-offs
- `cancel_reason_hint` останется derived signal, а не absolute truth; это приемлемо, пока UI явно показывает его как local diagnostic hint.
- transport milestones могут оказаться частично недоступны в middleware/`vscode-languageclient`; в этом случае реализация должна выбрать минимальный additional hook на client path, но не менять LSP contract.
- расширение feed увеличит плотность UI; webview/clipboard должны оставаться читабельными и не смешивать client diagnostics с server truth.

## Validation
- Focused extension tests MUST покрывать cancel-reason, supersession, transport-phase timestamps, result-shape и drift/overlap diagnostics.
- `npm run lint` в `vscode-extension/`.
- `openspec validate update-client-completion-probe-diagnostics --strict --no-interactive`.
