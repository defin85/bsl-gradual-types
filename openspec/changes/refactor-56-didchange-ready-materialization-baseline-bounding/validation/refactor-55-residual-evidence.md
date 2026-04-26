# Refactor-55 residual evidence

Source report:

```text
openspec/changes/refactor-55-didchange-ready-install-type-index-wait-bounding/validation/refactor-55-real-conf-big-diagnostics-representative-save-followup-bundle-live.json
```

The refactor-55 validation accepted the save-cycle exact type-index blocker:

```text
canonical_ready_install_type_index_resolution=approved
ready_install_exact_type_index_wait_contract_approved_count=4
ready_install_exact_type_index_wait_classified_blocker_count=4
ready_install_exact_type_index_wait_deadline_count=4
max_ready_install_exact_type_index_wait_elapsed_ms=1125
same_version_did_save_promotion_source_transition_count=4
```

The same report still failed the didChange materialization baseline:

```text
did_change_materialization_within_baseline=false
did_change_ready_snapshot_materialization_histogram_count=4
did_change_ready_snapshot_materialization_p50_ms=40311
did_change_ready_snapshot_materialization_p95_ms=40319
baseline_did_change_ready_snapshot_materialization_p50_ms=3226
baseline_did_change_ready_snapshot_materialization_p95_ms=3329
did_change_ready_snapshot_materialization_p50_vs_baseline_delta_ms=37085
did_change_ready_snapshot_materialization_p95_vs_baseline_delta_ms=36990
```

Representative cycle evidence shows the later save-cycle blocker is distinct
from the canonical ready snapshot that remains installed from the previous
didChange revision:

```text
followup_semantic_path_detached_ready_artifacts=4
canonical_ready_snapshot_state_after_terminal.file_version=2
observed_version_after_timeout=3
exact_ready_after_timeout=false
ready_install_exact_type_index_wait_terminal.snapshot_failure_reason=exact_type_index_deadline_before_ready_install
```

Conclusion: refactor-56 must fix or fail the pure didChange canonical
materialization baseline directly. A later save-cycle blocker classification is
useful evidence, but it must no longer make
`did_change_materialization_within_baseline=false` acceptable.
