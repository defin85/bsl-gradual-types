## Incident bundle evidence

Bundle:

```text
/home/egor/code/temp/bsl-observability-incident-2026-04-24T00-33-15Z
```

Captured at `2026-04-24T00:33:15.387Z`.

Build identity:

- extension: `0.4.159`
- lsp server: `0.4.159 (build: 2026-04-24 03:17:24, git: 070aa428)`
- server mode: `stdio`
- binary path:
  `/home/egor/code/bsl-gradual-types/vscode-extension/bin/lsp-server`
- binary mtime: `2026-04-24T00:26:32.131Z`

## Completion finding

Completion is not the primary bottleneck in this bundle:

- six completion traces were captured;
- client pre-write and transport/dispatch waits are in the `0-2ms` range for the captured
  requests;
- the only non-trivial completion request is dominated by local server `collect`, not
  client/transport ingress or response handoff.

## Diagnostics save finding

Both captured `didSave` cycles have the same residual shape:

| Version | Save cycle | First publish | Follow-up publish | Bounded winner | Lifecycle | Timeout leaf | Parser-base ms | Relief valve | Semantic path | Semantic query |
| ---: | ---: | ---: | ---: | --- | --- | --- | ---: | ---: | --- | ---: |
| 11 | 1 | 76ms | 8244ms | `timeout` at 3500ms | `started` | `parser_base_recovery` | 1008ms | 501ms timeout | `shadow_state` | 3488ms |
| 15 | 2 | 53ms | 8871ms | `timeout` at 3500ms | `started` | `parser_base_recovery` | 1910ms | 501ms timeout | `shadow_state` | 4110ms |

Additional facts:

- both cycles have `followup_ready_snapshot_task_state=in_flight_same_version`;
- both cycles have `followup_save_fastlane_gate_outcome=published`;
- both cycles have `followup_save_fastlane_gate_wait_ms=0`;
- both cycles use reused syntax work for the follow-up;
- both cycles fall back through `shadow_state` after timeout;
- cumulative metrics show `followup_semantic_path shadow_state=2`;
- cumulative metrics show `ready_snapshot_probe bounded_wait timeout=2`, `zero_budget not_ready=2`,
  and `relief_valve timeout=2`;
- cumulative metrics show `ready_snapshot_materialization source=did_save count=2 p50=4591ms`,
  but the per-cycle timeline does not tie that later materialization to each timed-out save
  family.

## Interpretation

This is not the old `refactor-50` waiting-only contour and not the p56 `refactor-51` admission
case. The producer reaches lifecycle `started`, but the runtime does not turn
`parser_base_recovery` into detached diagnostics-ready publication before the bounded follow-up
falls back to `shadow_state`.

The acceptance implication is a new fail gate:

```text
save_fastlane published
producer lifecycle started
timeout leaf parser_base_recovery
bounded wait timeout
semantic path shadow_state
semantic query dominated fallback
no per-cycle final producer lifecycle beyond started
```

The fix should bind the started same-version producer through parser-base recovery to detached
diagnostics-ready publication, or export a truthful per-cycle terminal reason that explains why the
same-family exact path was no longer safe to prefer.
