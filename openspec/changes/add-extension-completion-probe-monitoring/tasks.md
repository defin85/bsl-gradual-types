## 1. Spec and design
- [ ] 1.1 Обновить `bsl-intellisense` requirement для Observability completion UI так, чтобы authoritative `Server Timeline` и local `Client Probe Feed` отображались как две независимые поверхности без trace-level correlation.
- [ ] 1.2 Обновить `bsl-intellisense-v2` contract так, чтобы local `Client Probe Feed` оставался отдельным UI-level debug stream и не менял semantics server-generated timeline payload.
- [ ] 1.3 Зафиксировать в дизайне границы MVP: in-memory only, без raw source text, без нового backend protocol, без persistent telemetry и без trace-level correlation.
- [ ] 1.4 Явно вынести exact correlation через protocol-level `client_probe_id` в follow-up, а не в этот change.

## 2. Extension probe collection
- [ ] 2.1 Добавить bounded in-memory ring buffer для client-side completion probes с deterministic oldest-first eviction.
- [ ] 2.2 Инструментировать основной VS Code `LanguageClient` path, чтобы capture происходил на реально используемом extension entry point, а не только в optional enhanced client.
- [ ] 2.3 Зафиксировать bounded/redacted probe schema: `uri`, `document_version`, trigger metadata, локальные timing deltas, cancellation/result summary и только derived context flags.

## 3. UI and presentation
- [ ] 3.1 Расширить model/webview Observability completion UI отображением отдельного `Client Probe Feed`, не смешивая его со `Server Timeline`.
- [ ] 3.2 Добавить явные user-facing markers, что local probes являются local-only debug data и не эквивалентны server timeline.
- [ ] 3.3 Обновить clipboard/export formatting так, чтобы `Server Timeline` и `Client Probe Feed` были различимы и не создавали ложного впечатления общей причинно-следственной трассы.

## 4. Validation
- [ ] 4.1 Добавить extension tests на retention/eviction, redaction, корректный рендер двух независимых streams и отсутствие trace-level correlation.
- [ ] 4.2 Прогнать `npm run lint` в `vscode-extension/` и focused extension tests для completion timeline/probe flow.
- [ ] 4.3 Прогнать `openspec validate add-extension-completion-probe-monitoring --strict --no-interactive` и приложить команды acceptance.
