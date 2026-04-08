## 1. Implementation

- [x] 1.1 Add a failing regression for `didSave + idle_heavy` that currently recomputes expensive
      same-version syntax work despite already having reusable syntax artifacts.
- [x] 1.2 Implement same-version syntax artifact reuse in the `didSave` follow-up path.
- [x] 1.3 Keep diagnostics save timeline truthful for reused-vs-recomputed syntax work.
- [x] 1.4 Update bundle/summary projection if timeline shape changes.

## 2. Validation

- [x] 2.1 `cargo test -p bsl-backend --bin bsl-lsp-server diagnostics_save_timeline -- --nocapture`
- [x] 2.2 `cargo test -p bsl-backend --bin bsl-lsp-server did_save_fastlane -- --nocapture`
- [x] 2.3 `npm --prefix /home/egor/code/bsl-gradual-types/vscode-extension run compile:fast`
- [x] 2.4 `cd /home/egor/code/bsl-gradual-types/vscode-extension && BSL_TEST_GREP='LSP Custom Requests Test Suite|Observability Incident Bundle Test Suite|Observability Commands Test Suite' node ./out/test/runTest.js`
- [x] 2.5 `openspec validate refactor-08-diagnostics-save-followup-syntax-cost-reduction --strict --no-interactive`
- [x] 2.6 `CHANGE_ID=refactor-08-diagnostics-save-followup-syntax-cost-reduction cargo test -p bsl-backend --bin bsl-lsp-server p45_real_conf_big_did_save_diagnostics_followup_syntax_report_live -- --nocapture`
