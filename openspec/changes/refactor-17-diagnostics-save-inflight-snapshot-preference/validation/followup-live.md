# Follow-up Live Evidence

## Scenario

- Change: `refactor-17-diagnostics-save-inflight-snapshot-preference`
- Fixture: `file:///did_save_followup_inflight_exact_snapshot_fixture.bsl`
- Mutation: switch version `2` to semantic-only broken text `Сообщить(необъявленная);`
- Goal: prove that the same save cycle prefers an already-known exact in-flight snapshot over truthful `shadow_state`, but only for the exact-task-in-flight case.

## Command

`cargo test -p bsl-backend p7_did_save_followup_prefers_inflight_same_version_ready_snapshot_before_shadow_state -- --nocapture`

## Result

- Status: passed on `2026-04-12`
- Report: [refactor-17-diagnostics-save-inflight-snapshot-preference-did-save-followup-inflight-exact.json](/home/egor/code/bsl-gradual-types/backend/tests/perf/reports/refactor-17-diagnostics-save-inflight-snapshot-preference-did-save-followup-inflight-exact.json)
- `apply_delay_ms=4000`
- `did_change_parse_delay_ms=1200`
- `first_publish_elapsed_ms=22`
- `first_publish_syntax_only=true`
- `followup_publish_elapsed_ms=1233`
- `followup_ready_snapshot_task_state=in_flight_same_version`
- `followup_ready_snapshot_zero_probe=not_ready`
- `followup_ready_snapshot_wait_probe=ready`
- `followup_shadow_state_available=true`
- `followup_semantic_path=ready_artifacts`
- `followup_semantic_parse_source=snapshot`
- `followup_wait_for_file_version_ms=null`

## Verdict

The same save cycle now shows the intended ordering for the exact-task-in-flight case: the zero-budget probe misses, the bounded wait resolves to `ready`, and the eventual heavy follow-up publishes through `ready_artifacts` even though truthful `shadow_state` was already available. The companion regression `p7_did_save_followup_prefers_applied_state_when_writer_state_is_already_ready` remains green and proves that when no exact same-version task exists, the runtime still falls back immediately to `shadow_state` instead of introducing speculative waiting.
