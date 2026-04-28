# Incident Bundle Evidence: 2026-04-27T08-39-19Z

Source bundle:
`/home/egor/code/temp/bsl-observability-incident-2026-04-27T08-39-19Z`

Runtime source:

- `lsp_server` git: `033ac549`
- captured at: `2026-04-27T08:39:19.786Z`
- single URI bundle request count: `6`

## Integrity

- `intellisense_v2_observability_contract_violation_total=0`
- invalid saturation metric is absent
- `intellisense_v2_completion_fallback_unavailable_total=0`
- `intellisense_v2_completion_stale_fallback_total=0`
- `intellisense_v2_interactive_stale_served_total=0`

This keeps `refactor-57` and `refactor-58` integrity assumptions intact for the
new evidence.

## Speed Compared To Refactor-08

The checked-in `refactor-08` live report recorded:

- first publish: `102ms`
- `followup_wait_for_file_version_ms=39097`
- no full follow-up publish in the report

The new bundle records:

- first publish: `65ms` and `51ms`
- full follow-up publish: `1360ms` and `4346ms`

This confirms the user-observed speedup while still leaving a narrower
post-refactor-58 residual.

## Ready-Install Compared To Pre-Refactor-58

The pre-refactor-58 `2026-04-26T21-01-14Z` bundle localized the v15 residual to
ready-install contention:

- v15 `ready_install`: `2193ms`

The new post-refactor-58 bundle records:

- v15 `ready_install`: `1ms`
- v15 `snapshot_with_deps_ms`: `47ms`
- v15 `program_lowering_ms`: `3596ms`

This preserves the refactor-58 improvement and moves the remaining target to
exact assembly/program-lowering rather than ready-install.

## Completion Path

Completion is not the primary target for this change:

- completion traces: `6`
- `ok_non_empty=2`, `ok_empty=4`, `fail_closed=0`
- max client duration: `285ms`
- max server duration: `282ms`, dominated by normal `collect`
- max `service_future_to_first_poll_wait_ms`: `0ms`
- max `response_output_handoff_send_wait_ms`: `5ms`

## Current Context

The new refactor-58 current-context projection is present:

- current-context traces: `12`
- final status counts: `resolved=5`, `superseded=7`
- route counts: `broker_leader=9`, `ready_snapshot=3`
- ready-snapshot current-context wall times: `125-126ms`
- parser-coordinator leader parses: p50 `3807ms`, p95 `5063ms`

These long current-context parses are visible and attributable, but the
completion traces do not show corresponding ingress/egress blocking.

## didSave Follow-Up

Trace v11:

- first publish: `65ms`
- full follow-up publish: `1360ms`
- semantic path: `detached_ready_artifacts`
- `snapshot_with_deps_ms=628`
- `semantic_diagnostics_query_ms=731`
- `parse_exec_ms=15`
- `followup_readiness_blocker_bucket=snapshot_with_deps`

Trace v15:

- first publish: `51ms`
- full follow-up publish: `4346ms`
- semantic path: `detached_ready_artifacts`
- `snapshot_with_deps_ms=47`
- `semantic_diagnostics_query_ms=796`
- `ready_install_ms=1`
- `parse_exec_ms=3598`
- `exact_ready_snapshot_assembly_ms=3596`
- `program_conversion_ms=3596`
- `program_lowering_ms=3596`
- `timeout_phase=parse_exec`
- `timeout_leaf=program_lowering`
- `relief_valve_outcome=engaged_helped`
- `followup_readiness_blocker_bucket=snapshot_with_deps`
- raw backend timeline reuse evidence:
  - `program_lowering_reuse_outcome=full_rebuild`
  - `program_lowering_reused_lowering_units=0`
  - `program_lowering_rebuilt_lowering_units=2088`
  - `program_lowering_reuse_plan_build_source=null`
  - `program_lowering_reuse_plan_take_if_unique_hit=false`
  - `program_lowering_reuse_plan_borrowed_cache_hit=false`

The raw backend timeline already proved a full rebuild; the incident report lost
that proof before this change because the VS Code custom request and bundle
projection did not preserve the reuse fields.

## Scope Decision

The new change should target didSave exact assembly/program-lowering tail
boundedness and evidence completeness:

- do not reopen completion/UI/pre-send;
- do not reopen saturation integrity;
- do not reopen current-context attribution;
- do not treat `ready_install` as the remaining primary blocker;
- do not accept generic `snapshot_with_deps` attribution when measured
  `snapshot_with_deps_ms` is small and program lowering dominates.

These boundaries match the proposal non-goals for `refactor-57`, `refactor-58`,
completion/UI dispatch, current-context routing, and budget widening.

## Implementation Evidence: Instrumentation Projection

Initial implementation kept the backend diagnostics-save timeline as the source
of truth and closed the operator-export gap:

- backend timeline already carries program-lowering reuse outcome, rebuilt/reused
  unit counts, reuse-plan source, and reuse-plan hit flags;
- readiness blocker classification now prefers `program_lowering_tail` when the
  exact ready-snapshot timeout leaf/checkpoint is `program_lowering`, even if a
  small measured `snapshot_with_deps_ms` is also present;
- VS Code custom request typing now includes the program-lowering reuse fields;
- incident-bundle request summaries now preserve those fields into
  `incident.json` and render them in the human summary;
- incident-bundle gaps now fail visibly when a trace remains classified as
  generic `snapshot_with_deps` despite a `program_lowering` timeout leaf, or when
  a program-lowering tail lacks complete reuse evidence.

Runtime audit conclusion: the post-refactor-58 v15 residual is a real exact
program-lowering full rebuild (`0` reused units, `2088` rebuilt units) rather
than missing backend timeline instrumentation. The still-missing part is a
low-cardinality invalidation/source reason explaining why no reuse plan source
was available; until that exists, representative evidence must remain
fail-visible instead of treating the full rebuild as proven necessary.

## Implementation Evidence: Runtime Tail Bounding

The didSave exact rebuild seed now also accepts a latest ready parse snapshot
when it is already the same file version and same text as the requested didSave
version. This covers the post-didChange/post-refactor-58 lag shape where a ready
same-version snapshot exists, but didSave analysis state has not caught up yet;
the save-critical exact producer can reuse the already-ready AST seed instead of
starting from a cold/full lowering path.

If the ready snapshot is not usable, the remaining tail is no longer accepted as
generic readiness: the backend classifier reports `program_lowering_tail`, and
bundle projection requires the reuse outcome/units/hit flags to stay visible.

## Fresh Live Validation After Implementation

Fresh representative live report:

- command:
  `CHANGE_ID=refactor-59-didsave-program-lowering-tail-bounding BSL_V2_REAL_CONF_BIG_REPRESENTATIVE_SAVE_FOLLOWUP_BUNDLE_REPORT=openspec/changes/refactor-59-didsave-program-lowering-tail-bounding/validation/refactor-59-real-conf-big-diagnostics-representative-save-followup-bundle-live.json cargo test -p bsl-backend --bin bsl-lsp-server p56_real_conf_big_diagnostics_representative_save_followup_bundle_live -- --nocapture`
- result: `1 passed`, elapsed `261.97s`
- report:
  `validation/refactor-59-real-conf-big-diagnostics-representative-save-followup-bundle-live.json`

Report summary:

- representative save cycles: `4`
- `followup_semantic_path_detached_ready_artifacts=4`
- `followup_ready_snapshot_wait_probe_not_ready=4`
- `followup_ready_snapshot_wait_probe_timeout=0`
- `followup_ready_snapshot_program_lowering_full_rebuild_detached_ready_late_count=0`
- `followup_ready_snapshot_program_lowering_full_rebuild_shadow_state_later_detached_count=0`
- `save_fastlane_slow_first_publish_count=0`
- max first publish across cycles: `260ms`
- max first publish syntax query across cycles: `85ms`
- max follow-up publish across cycles: `1594ms`
- max follow-up ready-snapshot parse exec across cycles: `207ms`
- representative program-lowering reuse: `routine_body_reuse`, `2079` reused
  lowering units, `9` rebuilt lowering units
- `observability_contract_violation_total=0`
- invalid saturation metric absent
- didSave follow-up lane gauges remain present (`quota=1`, `active_slots=0`,
  `queue_depth=0`)

Fresh completion/current-context isolation checks:

- `cargo test -p bsl-backend --bin bsl-lsp-server p33_same_key_current_context_burst_keeps_completion_bounded_under_mixed_load -- --nocapture`
  passed. This guard delays same-key `bsl.getCurrentContext` work and asserts
  completion still returns within the `250ms` isolation budget, with
  `adapter_read_at_ms`, `service_future_to_first_poll_wait_ms`, and
  `response_output_handoff_send_wait_ms` also bounded by `250ms`.
- `cargo test -p bsl-backend --bin bsl-lsp-server current_context_timeline_`
  passed `current_context_timeline_can_filter_by_uri` and
  `current_context_timeline_retention_evicts_oldest_first`, preserving the
  current-context timeline API surface used by incident bundles.

Follow-up current-context ready-snapshot probe:

- `cargo test -p bsl-backend --bin bsl-lsp-server p33_get_current_context_uses_parse_snapshot_without_warming_exact_type_index -- --nocapture`
  now passes after allowing `bsl.getCurrentContext` to reuse a text-equivalent
  latest ready parse snapshot even when the shadow document has advanced to a
  newer same-text version without an exact type index. This restores the
  historical zero auxiliary parse assertion for current-context syntax-only
  resolution.
- `cargo test -p bsl-backend --bin bsl-lsp-server p33_get_current_context_ -- --nocapture --test-threads=1`
  passed `9` current-context guards. The serial mode is intentional for this
  slice because several tests share test-only global parse-attempt counters.

Focused checks:

- `cargo fmt --check`
- `npm run lint --prefix vscode-extension`
- `cargo test -p bsl-backend --bin bsl-lsp-server same_version_same_text_ready_snapshot_can_seed_didsave_rebuild`
- `cargo test -p bsl-backend --bin bsl-lsp-server diagnostics_save_timeline_classifies_program_lowering_tail_before_snapshot_with_deps`
- `cargo test -p bsl-backend --bin bsl-lsp-server p33_get_current_context_uses_parse_snapshot_without_warming_exact_type_index -- --nocapture`
- `cargo test -p bsl-backend --bin bsl-lsp-server p33_get_current_context_ -- --nocapture --test-threads=1`
- `cargo test -p bsl-backend --bin bsl-lsp-server p33_same_key_current_context_burst_keeps_completion_bounded_under_mixed_load -- --nocapture`
- `cargo test -p bsl-backend --bin bsl-lsp-server current_context_timeline_`
- `BSL_TEST_GREP="Observability Incident Bundle" npm test --prefix vscode-extension`
- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `openspec validate refactor-59-didsave-program-lowering-tail-bounding --strict --no-interactive`

An exploratory full `npm test --prefix vscode-extension -- --grep
"program-lowering tail"` also proved the Observability Incident Bundle suite
passed, but the runner ignores `--grep` and the full extension suite still hit
pre-existing Context/Stats timing failures unrelated to this change. The focused
`BSL_TEST_GREP` run above is the accepted projection evidence for this slice.
