## 1. Spec and design
- [ ] 1.1 Зафиксировать bounded vocabularies и optional semantics для cancel-reason, result-shape, transport-phase и overlap diagnostics.
- [ ] 1.2 Явно закрепить, что расширение probes не меняет `bsl.getCompletionTimeline` contract и не добавляет trace-level correlation.

## 2. Extension probe collection
- [ ] 2.1 Расширить bounded/redacted probe schema новыми cancellation diagnostics: `cancel_reason_hint`, optional `superseded_by_probe_id`, optional `superseded_after_ms`.
- [ ] 2.2 Добавить transport-phase diagnostics на default `LanguageClient` path так, чтобы можно было отделить client pre-send delay, LSP/in-flight wait и client post-response overhead.
- [ ] 2.3 Добавить result-shape diagnostics: bounded `result_kind`, bounded `item_count_bucket` и `is_incomplete`, когда этот сигнал доступен.
- [ ] 2.4 Добавить version-drift и overlap diagnostics: `document_version_at_terminal`, `did_change_count_during_probe`, `cursor_moved_during_probe`, `active_completion_count_at_start`, `same_uri_probe_overlap_count`, `newer_probe_started_before_terminal`.

## 3. UI and export
- [ ] 3.1 Обновить `Client Probe Feed` model/webview/clipboard так, чтобы новые diagnostics были видны пользователю как local-only debug data.
- [ ] 3.2 Сохранить fail-closed separation между enriched client probes и authoritative `Server Timeline`.

## 4. Validation
- [ ] 4.1 Добавить/обновить focused extension tests для cancel-reason, supersession, transport-phase timestamps, result-shape и version-drift/overlap diagnostics.
- [ ] 4.2 Прогнать `npm run lint` в `vscode-extension/` и focused extension tests для completion probe/timeline flow.
- [ ] 4.3 Прогнать `openspec validate update-client-completion-probe-diagnostics --strict --no-interactive`.
