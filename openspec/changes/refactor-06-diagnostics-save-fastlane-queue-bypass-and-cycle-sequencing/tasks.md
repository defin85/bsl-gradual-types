## 1. Implementation

- [x] 1.1 Добавить dedicated `save_cycle_sequence` для `didSave` lifecycle и пронести его через
      diagnostics save timeline DTO/backend state.
- [x] 1.2 Перевести `save_fastlane` shadow parse fallback на dedicated blocking path без shared
      interactive queue starvation.
- [x] 1.3 Обновить incident bundle / diagnostics save summary так, чтобы operator-facing ordering
      использовал `save_cycle_sequence`.
- [x] 1.4 Добавить regressions на queue starvation bypass и same-version save-cycle sequencing.

## 2. Validation

- [x] 2.1 `cargo test -p bsl-backend --bin bsl-lsp-server diagnostics_save_timeline -- --nocapture`
- [x] 2.2 `cargo test -p bsl-backend --bin bsl-lsp-server did_save_fastlane -- --nocapture`
- [x] 2.3 `npm --prefix /home/egor/code/bsl-gradual-types/vscode-extension run compile:fast`
- [x] 2.4 `cd /home/egor/code/bsl-gradual-types/vscode-extension && BSL_TEST_GREP='LSP Custom Requests Test Suite|Observability Incident Bundle Test Suite|Observability Commands Test Suite' node ./out/test/runTest.js`
- [x] 2.5 `openspec validate refactor-06-diagnostics-save-fastlane-queue-bypass-and-cycle-sequencing --strict --no-interactive`
