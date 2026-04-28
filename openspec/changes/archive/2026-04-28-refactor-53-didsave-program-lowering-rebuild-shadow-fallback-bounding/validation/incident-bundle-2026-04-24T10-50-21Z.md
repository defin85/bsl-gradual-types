## Incident bundle evidence

Bundle:

```text
/home/egor/code/temp/bsl-observability-incident-2026-04-24T10-50-21Z
```

Build identity:

```text
extension: 0.4.159
lsp_server: 0.4.159 (build: 2026-04-24 13:37:19, git: 00bcf03f)
```

Completion is not the primary bottleneck in this bundle:

- 5 completion traces.
- `adapter_to_dispatch_wait_ms=0-1`.
- `same_file_ingress_token_wait_ms=0`.
- `response_output_handoff_send_wait_ms=0`.
- Max completion duration is `195ms`, dominated by `collect`.

Diagnostics-save control trace:

```text
trace=diagnostics-save-trace-1
requested_version=11
save_cycle_sequence=1
first_publish=save_fastlane syntax_only published@69ms
followup_publish=idle_heavy full published@1404ms
followup_semantic_path=detached_ready_artifacts
followup_ready_snapshot_bounded_wait_winner=detached_ready_artifacts
followup_ready_snapshot_bounded_wait_elapsed_ms=631
followup_ready_snapshot_parse_exec_ms=40
program_lowering_ms=2
program_lowering_reuse_outcome=top_level_reuse
program_lowering_rebuilt_lowering_units=0
program_lowering_reused_lowering_units=2088
final_lifecycle=detached_diagnostics_ready_published
```

Diagnostics-save residual trace:

```text
trace=diagnostics-save-trace-2
requested_version=15
save_cycle_sequence=2
first_publish=save_fastlane syntax_only published@50ms
followup_publish=idle_heavy full published@8440ms
followup_publish.publish_wait_ms=1
followup_publish.semantic_diagnostics_query_ms=3679
followup_publish.semantic_diagnostics_parse_result_ms=3201
followup_semantic_path=shadow_state
followup_ready_snapshot_wait_probe=timeout
followup_ready_snapshot_bounded_wait_winner=timeout
followup_ready_snapshot_bounded_wait_elapsed_ms=3500
followup_ready_snapshot_timeout_phase=parse_exec
followup_ready_snapshot_timeout_leaf=program_lowering
followup_ready_snapshot_timeout_leaf_elapsed_ms=3550
followup_ready_snapshot_parse_exec_ms=3795
followup_ready_snapshot_exact_ready_snapshot_assembly_ms=3792
program_lowering_ms=3792
program_lowering_reuse_outcome=full_rebuild
program_lowering_rebuilt_lowering_units=2088
program_lowering_reused_lowering_units=0
program_lowering_reuse_plan_borrowed_cache_hit=false
program_lowering_reuse_plan_take_if_unique_hit=false
relief_valve_outcome=engaged_timed_out
relief_valve_elapsed_ms=501
lifecycle_at_timeout=admitted
final_lifecycle=detached_diagnostics_ready_published
```

Interpretation:

- The residual is backend `didSave` ready-snapshot/program-lowering behavior, not VS Code UI,
  completion transport ingress, response output handoff, or publish egress.
- The failing trace is not the older waiting-only `refactor-50` contour and not the
  `refactor-52` parser-base contour.
- The same-family producer eventually reached `detached_diagnostics_ready_published`; the observed
  problem is that bounded heavy follow-up had already timed out and published through `shadow_state`.
