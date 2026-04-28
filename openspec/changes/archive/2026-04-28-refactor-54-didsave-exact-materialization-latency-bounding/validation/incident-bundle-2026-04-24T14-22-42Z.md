## Incident bundle evidence

Bundle:

```text
/home/egor/code/temp/bsl-observability-incident-2026-04-24T14-22-42Z
```

Build identity:

```text
extension: 0.4.160
lsp_server: 0.4.160 (build: 2026-04-24 16:57:30, git: 00bcf03f)
```

Completion is not the primary bottleneck in this bundle:

- 6 completion traces.
- `client_before_transport_write_wait_ms=1-2`.
- `scheduler_poll_ready_wait_ms=0`.
- `admission_queue_wait_ms=0`.
- `same_file_ingress_token_wait_ms=0`.
- `response_output_handoff_send_wait_ms=0-1`.
- Max completion duration is `190ms`, dominated by `collect`.

Diagnostics-save trace 1:

```text
trace=diagnostics-save-trace-1
requested_version=11
save_cycle_sequence=1
first_publish=save_fastlane syntax_only published@3397ms
first_publish.syntax_diagnostics_query_ms=3397
first_publish.syntax_work_mode=recomputed
followup_publish=idle_heavy full published@577ms
followup_publish.semantic_diagnostics_query_ms=539
followup_semantic_path=detached_ready_artifacts
followup_ready_snapshot_bounded_wait_winner=detached_ready_artifacts
followup_ready_snapshot_bounded_wait_elapsed_ms=34
followup_ready_snapshot_wait_probe=not_ready
followup_ready_snapshot_parse_exec_ms=3933
followup_ready_snapshot_parser_base_recovery_ms=3926
program_lowering_ms=3
program_lowering_reuse_outcome=routine_body_reuse
program_lowering_rebuilt_lowering_units=2
program_lowering_reused_lowering_units=2086
program_lowering_reuse_plan_take_if_unique_hit=true
final_lifecycle=detached_diagnostics_ready_published
```

Diagnostics-save trace 2:

```text
trace=diagnostics-save-trace-2
requested_version=15
save_cycle_sequence=2
first_publish=save_fastlane syntax_only published@55ms
first_publish.syntax_diagnostics_query_ms=34
followup_publish=idle_heavy full published@4884ms
followup_publish.semantic_diagnostics_query_ms=700
followup_semantic_path=detached_ready_artifacts
followup_ready_snapshot_wait_probe=timeout
followup_ready_snapshot_bounded_wait_winner=timeout
followup_ready_snapshot_bounded_wait_elapsed_ms=3502
followup_ready_snapshot_timeout_phase=parse_exec
followup_ready_snapshot_timeout_leaf=program_lowering
followup_ready_snapshot_parse_exec_ms=4233
followup_ready_snapshot_exact_ready_snapshot_assembly_ms=4230
program_lowering_ms=4230
program_lowering_reuse_outcome=full_rebuild
program_lowering_rebuilt_lowering_units=2088
program_lowering_reused_lowering_units=0
program_lowering_reuse_plan_borrowed_cache_hit=false
program_lowering_reuse_plan_take_if_unique_hit=false
relief_valve_outcome=engaged_timed_out
relief_valve_elapsed_ms=501
final_lifecycle=detached_diagnostics_ready_published
```

Interpretation:

- The bundle does not reproduce the refactor-53 terminal `shadow_state` residual. Both
  diagnostics-save follow-ups use `detached_ready_artifacts`.
- The fresh residual is latency:
  - trace 1 has a slow first publish dominated by syntax diagnostics query while exact
    `parser_base_recovery` is also expensive;
  - trace 2 has a slow heavy follow-up because exact `program_lowering` performs a full rebuild and
    detached-ready arrives only after bounded wait and relief-valve timeouts.
- The bundle was captured from an installed `git 00bcf03f` binary. It is evidence for the new
  OpenSpec change, not acceptance evidence for the current dirty worktree.
