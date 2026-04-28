## Current-source residual evidence

Source report:

```text
backend/tests/perf/reports/refactor-54-didsave-exact-materialization-latency-bounding-real-conf-big-diagnostics-representative-save-followup-bundle-live.json
```

Refactor-54 accepted save-followup evidence remains bounded:

```text
followup_semantic_path_detached_ready_artifacts=4
followup_ready_snapshot_bounded_wait_winner_detached_ready_artifacts=4
followup_ready_snapshot_wait_probe_timeout=0
max_first_publish_elapsed_ms=223
max_first_publish_syntax_query_ms=73
max_followup_ready_snapshot_bounded_wait_elapsed_ms=47
max_followup_ready_snapshot_parse_exec_ms=163
max_followup_publish_elapsed_ms=2261
program_lowering_reuse_outcome=routine_body_reuse
program_lowering_reused_lowering_units=2079
program_lowering_rebuilt_lowering_units=9
```

Residual aggregate:

```text
did_change_ready_snapshot_materialization_histogram_count=4
did_change_ready_snapshot_materialization_p50_ms=42597
did_change_ready_snapshot_materialization_p95_ms=43758
```

Checked-in p56 baseline comparison:

```text
baseline_did_change_ready_snapshot_materialization_p50_ms=3226
baseline_did_change_ready_snapshot_materialization_p95_ms=3329
observed_vs_baseline_p50_delta_ms=39371
observed_vs_baseline_p95_delta_ms=40429
```

Representative cycle probes at follow-up timeout show the canonical path lagging the detached path:

```text
observed_version_after_timeout=stage2
ready_snapshot_state_after_timeout.file_version=stage1
ready_snapshot_state_after_timeout.source=DidChange
exact_ready_after_timeout=false
type_index_task_state_after_timeout.phase=computing
background_parse_task_state_after_timeout.phase=Some(Materializing)
type_index_parse_snapshot_meta_after_timeout=null
```

Code-order evidence:

```text
record_detached_diagnostics_ready_artifact_v2
wait_for_exact_type_index_before_ready_install_v2
record_ready_parse_snapshot_v2
record_intellisense_v2_ready_parse_snapshot_materialization
```

This explains why detached diagnostics-ready follow-up can be accepted while
`did_change_ready_snapshot_materialization_ms` remains high: the histogram includes canonical
ready-install waiting for exact type-index readiness after detached diagnostics-ready publication.

Source-attribution risk:

```text
source_label = background_parse_snapshot_apply_source_label(target.source)
```

The worker captures the label near loop start, while the scheduler can later promote or mutate a
same-version running `didChange` target to `DidSave`. Refactor-55 must preserve both
`original_source` and `effective_source` before using didChange histograms as source-class evidence.

## Implementation inspection: 3.1

The canonical ready-install lag is localized to the ready-parse-snapshot worker after detached
diagnostics-ready artifact publication:

```text
backend/src/bin/lsp_server/server/language_server/impl_document_sync.rs
record_detached_diagnostics_ready_artifact_v2
wait_for_exact_type_index_before_ready_install_v2
record_ready_parse_snapshot_v2
record_intellisense_v2_ready_parse_snapshot_materialization
```

`wait_for_exact_type_index_before_ready_install_v2` currently loops until exact type-index readiness,
retarget, supersession, or latest-version mismatch. It schedules/promotes type-index precompute and
sleeps between probes, but it has no checked-in deadline/envelope and no reportable trace for the
not-ready blocker class. In the representative p56 report, stage2 is the observed file version while
the canonical ready snapshot remains stage1 because the worker is parked in this exact-ready wait:

```text
observed_version_after_timeout=stage2
ready_snapshot_state_after_timeout.file_version=stage1
exact_ready_after_timeout=false
type_index_task_state_after_timeout.phase=computing
background_parse_task_state_after_timeout.phase=Some(Materializing)
type_index_parse_snapshot_meta_after_timeout=null
```

Root blocker class: canonical ready install is waiting for exact current type-index readiness after
detached diagnostics-ready publication. The detached path is already fast, so the residual is not a
refactor-54 detached-ready acceptance gap; it is an unbounded/unclassified canonical ready-install
wait included in `did_change_ready_snapshot_materialization_ms`.

## Implementation evidence: 3.2, 3.3, 3.7, 4.1

Code changes:

```text
backend/src/bin/lsp_server/server/language_server/impl_document_sync.rs
backend/src/bin/lsp_server/server/mod.rs
backend/src/bin/lsp_server/server/core/tests/live_reports/representative_bundle_live.rs
backend/src/bin/lsp_server/server/core/tests/did_save_followup/promotion_and_retarget.rs
```

`wait_for_exact_type_index_before_ready_install_v2` now starts a checked-in 5000ms ready-install
exact type-index envelope, records a ready-install wait snapshot on the task control, and returns a
reportable `deadline` outcome instead of sleeping indefinitely. The wait snapshot uses the same
low-cardinality shape as the existing interactive exact wait path where possible:
`waiter_action`, `matching_task_state`, and `task_phase`, plus ready-install-specific observed
version, exact readiness, canonical ready snapshot version, parse snapshot metadata, and
`blocker_class`.

Same-version didSave promotion now refreshes the effective target source before detached
diagnostics-ready publication, canonical ready install, materialization metrics, lifecycle terminal
labels, and phase metrics. Worker start remains original-source evidence, while final
materialization/phase labels use the effective source.

Targeted validation:

```text
cargo test -p bsl-backend --bin bsl-lsp-server p55_same_version_did_save_promotion_uses_effective_source_for_ready_install_metrics -- --nocapture
result: passed
```

## Implementation evidence: 3.4, 3.5, 3.6, 4.2, 4.3, 4.4

Representative live validation command:

```text
CHANGE_ID=refactor-55-didchange-ready-install-type-index-wait-bounding BSL_V2_REAL_CONF_BIG_REPRESENTATIVE_SAVE_FOLLOWUP_BUNDLE_REPORT=openspec/changes/refactor-55-didchange-ready-install-type-index-wait-bounding/validation/refactor-55-real-conf-big-diagnostics-representative-save-followup-bundle-live.json cargo test -p bsl-backend --bin bsl-lsp-server p56_real_conf_big_diagnostics_representative_save_followup_bundle_live -- --nocapture
result: passed
```

Report:

```text
openspec/changes/refactor-55-didchange-ready-install-type-index-wait-bounding/validation/refactor-55-real-conf-big-diagnostics-representative-save-followup-bundle-live.json
```

Key evidence from the report:

```text
followup_semantic_path_detached_ready_artifacts=4
followup_ready_snapshot_bounded_wait_winner_detached_ready_artifacts=4
followup_ready_snapshot_wait_probe_timeout=0
ready_install_exact_type_index_wait_classified_blocker_count=4
ready_install_exact_type_index_wait_deadline_count=4
ready_install_exact_type_index_wait_contract_approved_count=4
same_version_did_save_promotion_source_transition_count=4
canonical_ready_install_type_index_resolution=approved
max_ready_install_exact_type_index_wait_elapsed_ms=1125
did_change_ready_snapshot_materialization_p50_ms=40311
did_change_ready_snapshot_materialization_p95_ms=40319
did_change_materialization_within_baseline=false
```

The high `did_change_ready_snapshot_materialization_ms` values remain intentionally visible, but
the p56 gate now fails unless every cycle proves either canonical materialization or an approved
classified blocker. The passing refactor-55 report proves the latter: each cycle reaches
detached diagnostics-ready publication quickly, preserves original/effective source lineage
(`did_change` -> `did_save`), and then exports
`exact_type_index_deadline_before_ready_install` without installing a canonical ready snapshot for
the saved stage2 revision.

Targeted backend validation:

```text
cargo test -p bsl-backend --bin bsl-lsp-server p55_ready_install_exact_type_index_wait_snapshot_records_terminal_outcomes -- --nocapture
result: passed

cargo test -p bsl-backend --bin bsl-lsp-server p55_ready_install_exact_type_index_deadline_exports_classified_blocker -- --nocapture
result: passed

cargo test -p bsl-backend --bin bsl-lsp-server p55_same_version_did_save_promotion_uses_effective_source_for_ready_install_metrics -- --nocapture
result: passed

npm --prefix vscode-extension run compile:fast
result: passed

cargo check --workspace --all-targets
result: passed

cargo clippy --workspace --all-targets -- -D warnings
result: passed

openspec validate refactor-55-didchange-ready-install-type-index-wait-bounding --strict --no-interactive
result: passed
```
