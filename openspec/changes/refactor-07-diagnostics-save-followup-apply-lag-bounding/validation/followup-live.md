# Follow-up Live Evidence

## Scenario

- Change: `refactor-07-diagnostics-save-followup-apply-lag-bounding`
- Fixture: `examples/conf_big/Documents/РеализацияТоваровУслуг/Forms/ФормаДокументаОбщая/Ext/Form/Module.bsl`
- Mutation: append broken suffix `Процедура SaveFollowupBroken(`
- Goal: prove that `didSave` keeps the bounded syntax-only first publish and that the heavy
  follow-up becomes request-centrically attributable as `apply_lag` instead of opaque `pending`.

## Command

`CHANGE_ID=refactor-07-diagnostics-save-followup-apply-lag-bounding cargo test -p bsl-backend --bin bsl-lsp-server p44_real_conf_big_did_save_diagnostics_followup_report_live -- --nocapture`

## Result

- Status: passed on `2026-04-08`
- Report: [refactor-07-diagnostics-save-followup-apply-lag-bounding-real-conf-big-did-save-diagnostics-followup-live.json](/home/egor/code/bsl-gradual-types/backend/tests/perf/reports/refactor-07-diagnostics-save-followup-apply-lag-bounding-real-conf-big-did-save-diagnostics-followup-live.json)
- `apply_delay_ms=4000`
- `first_publish_budget_ms=2500`
- `first_publish_elapsed_ms=135`
- `first_publish_version=2`
- `first_publish_diagnostics_count=1`
- `first_publish_syntax_only=true`
- `save_fastlane_published_total=1`
- `followup_wait_reason=apply_lag`
- `save_cycle_sequence=1`

## Verdict

The `didSave` fastlane stays comfortably within the bounded first-publish budget
(`135ms < 2500ms`) even under injected delayed apply (`4000ms`), and the remaining heavy
follow-up does not hide behind opaque `pending`. The live request-centric trace reports
`followup_wait_reason=apply_lag` for the same save cycle, which matches the contract of
`refactor-07`.
