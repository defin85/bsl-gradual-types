## 1. Contract

- [x] 1.1 Add `bsl-intellisense-v2` requirements for canonical ready-install/type-index wait after
      detached diagnostics-ready publication.
- [x] 1.2 Add requirements for original-source vs effective-source materialization attribution when
      a same-version didSave promotes or mutates a running didChange target.
- [x] 1.3 Record current-source residual evidence and keep refactor-54 save-followup acceptance out
      of scope.

## 2. Design

- [x] 2.1 Define the boundary between detached diagnostics-ready publication and canonical live
      ready snapshot install.
- [x] 2.2 Define low-cardinality observability for
      `wait_for_exact_type_index_before_ready_install_v2`, type-index task state, exact readiness,
      parse snapshot metadata, and ready snapshot state.
- [x] 2.3 Define how materialization metrics choose effective source while preserving original
      source and promotion/retarget evidence.

## 3. Implementation

- [x] 3.1 Inspect the canonical ready-install path and identify why the representative p56 stage2
      target remains exact-not-ready while stage1 remains the canonical ready snapshot.
- [x] 3.2 Add direct instrumentation around exact type-index wait before canonical ready install,
      including elapsed wait, explicit ceiling/deadline class, outcome, task phase, active requested
      version, exact readiness, current ready snapshot version, and parse snapshot
      metadata/blocker class.
- [x] 3.3 Fix source attribution so final ready-parse-snapshot materialization metrics use the
      effective target source after didSave promotion/retarget, while preserving original source as
      evidence; include lifecycle terminal labels and phase metrics in the same attribution fix.
- [x] 3.4 Implement the root fix so representative p56 canonical ready install either reaches
      exact type-index readiness inside an explicit checked-in envelope derived from baseline
      materialization evidence or exports a truthful classified blocker without weakening exact
      gates.
- [x] 3.5 Extend incident-bundle and representative live-report projections for detached
      diagnostics-ready elapsed, canonical ready-install wait elapsed/outcome, original/effective
      source, promotion/retarget event, type-index task state, and final canonical source.
- [x] 3.6 Update p56 gates so high canonical materialization latency fails unless the report proves
      a contract-approved blocker or a truthful supersession/cancellation/retarget.
- [x] 3.7 Prefer reusing or extending the existing bounded exact type-index wait trace shape used by
      interactive exact consumers; do not leave `wait_for_exact_type_index_before_ready_install_v2`
      as an unbounded sleep loop with no reportable deadline/envelope.

## 4. Validation

- [x] 4.1 Run targeted backend tests for same-version didSave promotion of a didChange worker and
      verify original/effective source attribution.
- [x] 4.2 Run targeted backend tests for exact type-index ready-install wait outcomes:
      ready, retargeted, superseded, latest-version mismatch, and blocked/not-ready.
- [x] 4.3 Run representative live validation on `examples/conf_big` and verify p56 reports bounded
      canonical ready-install/type-index wait or a truthful classified blocker.
- [x] 4.4 Run relevant diagnostics-save/didChange live-report regression tests to ensure
      refactor-54 detached diagnostics-ready acceptance remains green.
- [x] 4.5 Run `cargo check --workspace --all-targets` and
      `cargo clippy --workspace --all-targets -- -D warnings` if production Rust changes.
- [x] 4.6 Run
      `openspec validate refactor-55-didchange-ready-install-type-index-wait-bounding --strict --no-interactive`.

## 5. Initial Evidence

- [x] 5.1 Current-source report:
      `backend/tests/perf/reports/refactor-54-didsave-exact-materialization-latency-bounding-real-conf-big-diagnostics-representative-save-followup-bundle-live.json`.
- [x] 5.2 Residual aggregate: `did_change_ready_snapshot_materialization_ms p50=42597`,
      `p95=43758`, `count=4`.
- [x] 5.2a Checked-in p56 baseline: `did_change_ready_snapshot_materialization_ms p50=3226`,
      `p95=3329`; validation must report observed-vs-baseline comparison or equivalent pass/fail
      evidence.
- [x] 5.3 Fast accepted save-followup path: `followup_semantic_path_detached_ready_artifacts=4`,
      `max_followup_ready_snapshot_bounded_wait_elapsed_ms=47`,
      `max_followup_ready_snapshot_parse_exec_ms=163`,
      `max_followup_publish_elapsed_ms=2261`.
- [x] 5.4 Cycle probes: observed stage2 version while canonical ready snapshot remains stage1,
      `exact_ready_after_timeout=false`, type-index task phase `computing`, background parse task
      phase `Some(Materializing)`, and missing type-index parse snapshot metadata.
