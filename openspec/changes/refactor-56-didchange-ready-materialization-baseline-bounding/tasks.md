## 1. Contract

- [x] 1.1 Add `bsl-intellisense-v2` requirements for pure didChange canonical
      ready materialization to stay within the checked-in p56 baseline.
- [x] 1.2 Add requirements for successful materialization metrics to exclude
      classified ready-install blockers and didSave-promoted/save-cycle work
      from the pure didChange baseline view.
- [x] 1.3 Record refactor-55 residual evidence and explicitly state why the
      save-cycle exact type-index blocker no longer satisfies the didChange
      baseline contract.

## 2. Design

- [x] 2.1 Inspect the p56 stage1 pure didChange path and identify where the
      roughly 40s materialization wait is spent.
- [x] 2.2 Define the non-save-cycle didChange ready-install/type-index wait
      envelope and report fields, reusing the refactor-55 probe shape.
- [x] 2.3 Define metric/report semantics for successful pure didChange samples,
      promoted save-cycle samples, classified non-success blockers, and excluded
      samples.

## 3. Implementation

- [x] 3.1 Add ready-install exact type-index wait tracing for pure didChange
      canonical installs, including elapsed wait, ceiling/deadline class,
      active requested version, observed version, current ready snapshot version,
      exact readiness, type-index task phase, parse snapshot metadata state, and
      terminal outcome.
- [x] 3.2 Fix the pure didChange current-revision readiness path so the
      representative p56 canonical ready install reaches exact type-index
      readiness within the checked-in baseline without weakening exact gates.
- [x] 3.3 Ensure non-success didChange blockers are not recorded as successful
      `did_change_ready_snapshot_materialization_ms` samples.
- [x] 3.4 Extend the representative p56 report with successful pure didChange
      sample counts, excluded/blocker counts, promotion/save-cycle counts, and
      observed-vs-baseline comparison.
- [x] 3.5 Tighten the p56 gate so `did_change_materialization_within_baseline`
      must be true for accepted current-source validation; a later save-cycle
      blocker classification must not mask this failure.
- [x] 3.6 Update incident-bundle or runtime metric projections if new
      low-cardinality terminal reasons or sample classes are introduced.

## 4. Validation

- [x] 4.1 Add targeted backend coverage for pure didChange ready-install wait
      outcomes: ready inside baseline, superseded/cancelled, latest-version
      mismatch, and current blocker/deadline as non-success.
- [x] 4.2 Add targeted metric/report coverage proving classified blockers and
      didSave-promoted save-cycle work do not count as successful pure didChange
      materialization samples.
- [x] 4.3 Run representative p56 live validation on `examples/conf_big` and
      verify `did_change_materialization_within_baseline=true`.
- [x] 4.4 Re-run refactor-55 focused coverage to ensure save-cycle blocker
      classification and effective source attribution remain intact.
- [x] 4.5 Run `cargo check --workspace --all-targets` and
      `cargo clippy --workspace --all-targets -- -D warnings` if production Rust
      changes.
- [x] 4.6 Run
      `openspec validate refactor-56-didchange-ready-materialization-baseline-bounding --strict --no-interactive`.

## 5. Initial Evidence

- [x] 5.1 Refactor-55 p56 report:
      `openspec/changes/refactor-55-didchange-ready-install-type-index-wait-bounding/validation/refactor-55-real-conf-big-diagnostics-representative-save-followup-bundle-live.json`.
- [x] 5.2 Residual contract result:
      `did_change_materialization_within_baseline=false`.
- [x] 5.3 Residual aggregate:
      `did_change_ready_snapshot_materialization_ms p50=40311`, `p95=40319`,
      `count=4`, compared to baseline `p50=3226`, `p95=3329`.
- [x] 5.4 Refactor-55 save-cycle classification evidence:
      `canonical_ready_install_type_index_resolution=approved`,
      `ready_install_exact_type_index_wait_classified_blocker_count=4`,
      `ready_install_exact_type_index_wait_deadline_count=4`.
