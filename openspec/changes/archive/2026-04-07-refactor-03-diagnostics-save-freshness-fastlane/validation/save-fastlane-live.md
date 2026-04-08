# Save Fastlane Live Evidence

## Scenario

- Change: `refactor-03-diagnostics-save-freshness-fastlane`
- Fixture: `examples/conf_big/Documents/РеализацияТоваровУслуг/Forms/ФормаДокументаОбщая/Ext/Form/Module.bsl`
- Mutation: append broken suffix `Процедура SaveFastlaneBroken(`
- Goal: prove that the first diagnostics refresh after `didSave` no longer waits for delayed writer apply.

## Command

`CHANGE_ID=refactor-03-diagnostics-save-freshness-fastlane cargo test -p bsl-backend --bin bsl-lsp-server p43_real_conf_big_did_save_diagnostics_fastlane_report_live -- --nocapture`

## Result

- Status: passed on `2026-04-07`
- Report: [refactor-03-diagnostics-save-freshness-fastlane-real-conf-big-did-save-diagnostics-fastlane-live.json](/home/egor/code/bsl-gradual-types/backend/tests/perf/reports/refactor-03-diagnostics-save-freshness-fastlane-real-conf-big-did-save-diagnostics-fastlane-live.json)
- `apply_delay_ms=4000`
- `first_publish_budget_ms=2500`
- `first_publish_elapsed_ms=84`
- `first_publish_version=2`
- `first_publish_diagnostics_count=1`
- `first_publish_syntax_only=true`
- `save_fastlane_published_total=1`

## Verdict

The first publish after `didSave` beats the injected apply lag by a wide margin (`84ms < 4000ms`),
stays within the bounded first-publish budget (`84ms < 2500ms`), and remains same-version truthful
(`version=2`, syntax-only publish for the saved broken text).
