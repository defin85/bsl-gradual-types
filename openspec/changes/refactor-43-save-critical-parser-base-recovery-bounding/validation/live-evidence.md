# Validation Evidence

## Commands Run

### Parser-base scoped runtime/backend regressions

- `cargo test -p bsl-runtime tree_cache_prime_options_can_skip_optional_ast_priming_when_reuse_ast_is_preseeded -- --nocapture`
- `cargo test -p bsl-backend --bin bsl-lsp-server p27_diagnostics_save_timeline_reports_parser_base_recovery_checkpoint_for_exact_worker -- --nocapture`
- `cargo test -p bsl-backend --bin bsl-lsp-server p22_did_change_stale_parser_base_distinguishes_missing_matching_ready_snapshot -- --nocapture`
- `cargo test -p bsl-backend --bin bsl-lsp-server p22_did_change_stale_parser_base_distinguishes_tree_cache_mismatch_after_prime -- --nocapture`
- `cargo test -p bsl-backend --bin bsl-lsp-server p7_promote_type_index_precompute_restarts_completed_phase_task_without_exact_ready -- --nocapture`
- `cargo test -p bsl-backend --bin bsl-lsp-server same_revision_ready_snapshot_waits_for_exact_type_index_before_hover -- --nocapture`
- `cargo test -p bsl-backend --bin bsl-lsp-server p33_get_current_context_briefly_waits_for_equivalent_snapshot_worker_before_broker_parse -- --nocapture`

### Extension incident-bundle fidelity verification

- `npm --prefix ./vscode-extension run compile:fast`
- `BSL_TEST_GREP='getDiagnosticsSaveTimeline should work via executeCommand|happy path bundle should contain request-centric incident report and all raw attachments|diagnostics save summary should render active cycle as in_flight with pending followup|v20 diagnostics save timeline should mark timeout-leaf fidelity as unavailable by design' npm --prefix ./vscode-extension test`

## Passing Results

- The parser-base recovery runtime regression passed and proved that save-critical tree-cache
  recovery can skip optional AST priming when reuse AST is already preseeded.
- The diagnostics-save timeline regression passed and preserved
  `followup_ready_snapshot_timeout_leaf=parser_base_recovery` plus the matching checkpoint fields.
- The stale-parser-base root-cause regressions passed for:
  - missing matching ready snapshot;
  - tree-cache mismatch after prime.
- The type-index waiter promotion regression passed and preserved fail-closed behavior when a
  retained completed same-version task has no exact-ready artifact yet.
- The same-revision hover wait regression passed and preserved the canonical live exact gate.
- The same-version current-context wait regression passed and still resolved through
  `ready_snapshot` instead of launching an independent broker parse once the equivalent snapshot
  worker won within the bounded wait.
- The extension verification passed:
  - `getDiagnosticsSaveTimeline should work via executeCommand`
  - `happy path bundle should contain request-centric incident report and all raw attachments`
  - `diagnostics save summary should render active cycle as in_flight with pending followup`
  - `v20 diagnostics save timeline should mark timeout-leaf fidelity as unavailable by design`

## Representative Handoff Evidence

`refactor-43` no longer owns the late same-version `ready_install` remediation.
That branch was handed off to `refactor-44-save-followup-detached-ready-artifacts`.

Two live repros confirm why:

### `p55_real_conf_big_diagnostics_ready_snapshot_leaf_report_live`

The command

- `CHANGE_ID=refactor-43-save-critical-parser-base-recovery-bounding BSL_V2_REAL_CONF_BIG_READY_SNAPSHOT_LEAF_REPORT=openspec/changes/refactor-43-save-critical-parser-base-recovery-bounding/validation/refactor-43-real-conf-big-diagnostics-ready-snapshot-leaf-live.json cargo test -p bsl-backend --bin bsl-lsp-server p55_real_conf_big_diagnostics_ready_snapshot_leaf_report_live -- --nocapture`

did not reach the old `idle_heavy -> ready_artifacts` publish expectation.
Its last observed trace was:

- `followup_ready_snapshot_parse_exec_ms=165`
- `followup_ready_snapshot_parse_exec_core_build_parser_tree_build_ms=7`
- `followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_ms=152`
- `followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_ms=152`
- `followup_ready_snapshot_dominant_phase=ready_install`
- `followup_ready_snapshot_ready_install_ms=3947`
- `followup_ready_snapshot_timeout_phase=ready_install`
- `followup_ready_snapshot_timeout_leaf=ready_install`
- `followup_ready_snapshot_wait_probe=timeout`
- `followup_ready_snapshot_relief_valve_outcome=engaged_timed_out`
- `followup_ready_snapshot_continuation_reason=exhausted_continuation_proof`
- `followup_semantic_path=shadow_state`

This is exactly the evidence used to hand the remaining late-install branch off to `refactor-44`
instead of treating it as unfinished parser-base work inside `refactor-43`.

### `p49_real_conf_big_stale_parser_base_root_cause_report_live`

The command

- `CHANGE_ID=refactor-43-save-critical-parser-base-recovery-bounding BSL_V2_REAL_CONF_BIG_STALE_PARSER_BASE_ROOT_CAUSE_REPORT=openspec/changes/refactor-43-save-critical-parser-base-recovery-bounding/validation/refactor-43-real-conf-big-stale-parser-base-root-cause-live.json cargo test -p bsl-backend --bin bsl-lsp-server p49_real_conf_big_stale_parser_base_root_cause_report_live -- --nocapture`

did not reproduce the old lagging-shadow precondition.
It timed out waiting for the historical state where shadow advanced to `v3` while the ready
snapshot was still stuck at `v1`.

That failure mode is consistent with the current tree: the old parser-base-dominant setup is no
longer the representative residual on this path.

## Operational Probe Note

The legacy `p53_real_conf_big_exact_program_lowering_report_live` probe is intentionally not part
of the active operational gate for `refactor-43` or `refactor-44`.

It remains useful only as an optional diagnostic probe if someone needs to test whether the
residual moved back into `program_lowering`. The current active acceptance split is:

- parser-base scoped regressions and timeout-leaf fidelity for `refactor-43`;
- late `ready_install` save-followup traces (`p55` / `p56`-style evidence) for `refactor-44`.

## Additional Note

The older smoke

- `cargo test -p bsl-backend --bin bsl-lsp-server p33_get_current_context_uses_parse_snapshot_without_warming_exact_type_index -- --nocapture`

currently fails on this tree with `parse_attempts=1`.

This did not block `refactor-43` closure because the scoped downstream acceptance needed here is
covered by `p33_get_current_context_briefly_waits_for_equivalent_snapshot_worker_before_broker_parse`,
which still proves the same-version `ready_snapshot` path after bounded worker reuse.
