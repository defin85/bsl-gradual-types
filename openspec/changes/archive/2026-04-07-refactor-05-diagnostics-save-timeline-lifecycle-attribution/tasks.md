## 1. Backend lifecycle
- [x] 1.1 Добавить bounded terminal-key suppression для `diagnostics_save_timeline`, чтобы late result не воскрешал duplicate trace после terminal archive.
- [x] 1.2 Покрыть overlapping / late-result regression test на один `(requested_version, diagnostics_generation)`.

## 2. Backend attribution
- [x] 2.1 Добавить optional `blocking_queue_wait_ms` в diagnostics save timeline publish trace и поднять contract version.
- [x] 2.2 Протянуть queue-wait attribution в `save_fastlane` fallback path и закрыть regression тестом с injected queue delay.

## 3. Bundle projection
- [x] 3.1 Обновить extension DTO/rendering так, чтобы active cycle рендерился как `in_flight`, а pending profile как `pending`.
- [x] 3.2 Добавить bundle/request tests на новый rendering и live contract compatibility.

## 4. Validation
- [x] 4.1 `cargo test -p bsl-backend --bin bsl-lsp-server diagnostics_save_timeline -- --nocapture`
- [x] 4.2 `npm --prefix /home/egor/code/bsl-gradual-types/vscode-extension run compile:fast`
- [x] 4.3 `cd /home/egor/code/bsl-gradual-types/vscode-extension && BSL_TEST_GREP='LSP Custom Requests Test Suite|Observability Incident Bundle Test Suite|Observability Commands Test Suite' node ./out/test/runTest.js`
- [x] 4.4 `openspec validate refactor-05-diagnostics-save-timeline-lifecycle-attribution --strict --no-interactive`
