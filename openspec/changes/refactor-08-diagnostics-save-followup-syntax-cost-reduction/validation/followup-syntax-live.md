# Follow-up Syntax Live Evidence

## Scenario

- Change: `refactor-08-diagnostics-save-followup-syntax-cost-reduction`
- Fixture: `examples/conf_big/Documents/РеализацияТоваровУслуг/Forms/ФормаДокументаОбщая/Ext/Form/Module.bsl`
- Mutation: append broken suffix `Процедура SaveFollowupSyntaxBroken(`
- Goal: prove that `didSave` keeps a truthful syntax-only first publish and that the same save
  cycle exposes `idle_heavy` syntax reuse instead of redundant full syntax recompute.

## Command

`CHANGE_ID=refactor-08-diagnostics-save-followup-syntax-cost-reduction cargo test -p bsl-backend --bin bsl-lsp-server p45_real_conf_big_did_save_diagnostics_followup_syntax_report_live -- --nocapture`

## Result

- Status: passed on `2026-04-08`
- Report: [refactor-08-diagnostics-save-followup-syntax-cost-reduction-real-conf-big-did-save-diagnostics-followup-syntax-live.json](/home/egor/code/bsl-gradual-types/backend/tests/perf/reports/refactor-08-diagnostics-save-followup-syntax-cost-reduction-real-conf-big-did-save-diagnostics-followup-syntax-live.json)
- `first_publish_elapsed_ms=102`
- `first_publish_version=2`
- `first_publish_diagnostics_count=1`
- `first_publish_syntax_only=true`
- `save_fastlane_published_total=1`
- `followup_syntax_work_mode=reused`
- `followup_wait_reason=semantic_work`
- `followup_wait_for_file_version_ms=39097`
- `followup_snapshot_with_deps_ms=0`
- `followup_syntax_diagnostics_query_ms=null`

## Verdict

The live `conf_big` save cycle now exposes same-version syntax reuse for `idle_heavy` only once the
follow-up has actually entered the semantic path. The report stays truthful: the first publish
remains syntax-only, and the in-flight follow-up reports `followup_syntax_work_mode=reused`
together with `followup_wait_reason=semantic_work`, `followup_wait_for_file_version_ms=39097`,
and no recomputed `followup_syntax_diagnostics_query_ms`. This is explicit proof that syntax reuse
was applied before final publish became available, which matches the contract of `refactor-08`.
