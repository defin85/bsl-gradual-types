## 1. Implementation

- [x] 1.1 Добавить failing regression на `didSave`, где `save_fastlane` уже published, а
      `idle_heavy` застревает на delayed apply / `wait_for_file_version`.
- [x] 1.2 Реализовать same-version follow-up path, который предпочитает ready artifacts и не
      делает apply-lag primary gate по умолчанию.
- [x] 1.3 Расширить diagnostics save timeline request-centric attribution для pending heavy
      follow-up.
- [x] 1.4 Обновить incident bundle / summary projection для нового follow-up attribution.

## 2. Validation

- [x] 2.1 `cargo test -p bsl-backend --bin bsl-lsp-server diagnostics_save_timeline -- --nocapture`
- [x] 2.2 `cargo test -p bsl-backend --bin bsl-lsp-server did_save_fastlane -- --nocapture`
- [x] 2.3 `npm --prefix /home/egor/code/bsl-gradual-types/vscode-extension run compile:fast`
- [x] 2.4 `cd /home/egor/code/bsl-gradual-types/vscode-extension && BSL_TEST_GREP='LSP Custom Requests Test Suite|Observability Incident Bundle Test Suite|Observability Commands Test Suite' node ./out/test/runTest.js`
- [x] 2.5 `openspec validate refactor-07-diagnostics-save-followup-apply-lag-bounding --strict --no-interactive`
- [x] 2.6 `CHANGE_ID=refactor-07-diagnostics-save-followup-apply-lag-bounding cargo test -p bsl-backend --bin bsl-lsp-server p44_real_conf_big_did_save_diagnostics_followup_report_live -- --nocapture`
