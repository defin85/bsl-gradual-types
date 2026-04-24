## p56 representative conf_big live gate

Command:

```bash
CHANGE_ID=refactor-51-didsave-exact-producer-lane-bounding \
BSL_V2_REAL_CONF_BIG_REPRESENTATIVE_SAVE_FOLLOWUP_BUNDLE_REPORT=backend/tests/perf/reports/refactor-51-didsave-exact-producer-lane-bounding-real-conf-big-diagnostics-representative-save-followup-bundle-live.json \
cargo test -p bsl-backend p56_real_conf_big_diagnostics_representative_save_followup_bundle_live -- --nocapture
```

Latest passing result: 2026-04-24, `1 passed`, finished in 405.81s.

Report:

```text
backend/tests/perf/reports/refactor-51-didsave-exact-producer-lane-bounding-real-conf-big-diagnostics-representative-save-followup-bundle-live.json
```

Summary:

- 4/4 cycles used `followup_semantic_path=detached_ready_artifacts`.
- 4/4 cycles used `followup_ready_snapshot_bounded_wait_winner=detached_ready_artifacts`.
- 4/4 cycles exported `followup_did_save_exact_producer_lifecycle_state=detached_diagnostics_ready_published`.
- 0 cycles used `followup_semantic_path=shadow_state`.
- 0 bounded waits timed out.
- `max_first_publish_elapsed_ms=86`.
- `max_followup_save_fastlane_gate_wait_ms=0`.
- `max_followup_ready_snapshot_bounded_wait_elapsed_ms=92`.
- `max_followup_publish_elapsed_ms=1171`, below the representative detached publish ceiling of 5219ms.
- `max_followup_publish_semantic_diagnostics_query_ms=1101`.
- `max_followup_publish_snapshot_with_deps_ms=93`.
- `max_followup_publish_publish_wait_ms=3`.

Cycle evidence:

| Cycle | First publish ms | Gate wait ms | Follow-up publish ms | Semantic query ms | Snapshot/deps ms | Non-query residual ms | Path | Winner | Lifecycle |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | --- | --- | --- |
| 1 | 85 | 0 | 1171 | 1101 | 64 | 70 | `detached_ready_artifacts` | `detached_ready_artifacts` | `detached_diagnostics_ready_published` |
| 2 | 71 | 0 | 1011 | 946 | 59 | 65 | `detached_ready_artifacts` | `detached_ready_artifacts` | `detached_diagnostics_ready_published` |
| 3 | 81 | 0 | 1164 | 1063 | 93 | 101 | `detached_ready_artifacts` | `detached_ready_artifacts` | `detached_diagnostics_ready_published` |
| 4 | 86 | 0 | 1045 | 965 | 1 | 80 | `detached_ready_artifacts` | `detached_ready_artifacts` | `detached_diagnostics_ready_published` |

Interpretation: the representative same-version `didSave` contour now reaches detached
diagnostics-ready publication as the bounded winner for every captured cycle. The imported
`refactor-50` waiting-phase `shadow_state` fail gate is closed for this run, and the previous
post-detached follow-up publish residual is no longer present.
