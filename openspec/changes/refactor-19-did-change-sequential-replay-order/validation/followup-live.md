# Follow-up Live Evidence

## Scenario

- Change: `refactor-19-did-change-sequential-replay-order`
- Fixture: real `conf_big` module
  [Module.bsl](/home/egor/code/bsl-gradual-types/examples/conf_big/Documents/РеализацияТоваровУслуг/Forms/ФормаДокументаОбщая/Ext/Form/Module.bsl)
- Mutation: one `textDocument/didChange` notification with two sequential ranged inserts at EOF on version `2`
- Goal: prove that a live same-file ranged `didChange` on `conf_big` now stays incremental under LSP receive-order semantics instead of false-fallbacking to `edits_do_not_match_new_content`

## Command

`cargo test -p bsl-backend --bin bsl-lsp-server p47_real_conf_big_sequential_ranged_did_change_report_live -- --nocapture`

## Result

- Status: passed on `2026-04-12`
- Report: [refactor-19-did-change-sequential-replay-order-real-conf-big-sequential-ranged-did-change-live.json](/home/egor/code/bsl-gradual-types/backend/tests/perf/reports/refactor-19-did-change-sequential-replay-order-real-conf-big-sequential-ranged-did-change-live.json)
- `requested_version=2`
- `parse_mode=incremental`
- `base_text_source=shadow_state`
- `change_shape=ranged`
- `content_changes_count=2`
- `replay_order=receive_order`
- `base_document_version=1`
- `changed_ranges_count=2`
- `fallback_reason=null`

## Verdict

The live target scenario now exhibits the intended contract: a same-file sequential ranged
`didChange` on the real `conf_big` module materializes version-bound parse-snapshot evidence with
`parse_mode=incremental` and no canonical fallback reason. The new evidence fields also make the
producer attribution explicit in the report: the batch used `replay_order=receive_order` and the
base text came from `shadow_state` version `1`, so this path no longer needs log inspection to
distinguish replay drift from stale-base drift.
