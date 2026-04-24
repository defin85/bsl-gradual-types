## 1. Contract

- [x] 1.1 Add a `bsl-intellisense-v2` requirement for rebuild-dominated same-version `didSave`
      follow-up: `parse_exec/program_lowering full_rebuild` MUST NOT silently terminate through
      `shadow_state` when the same save family later publishes detached diagnostics-ready.
- [x] 1.2 Add representative acceptance that fails on
      `program_lowering full_rebuild -> bounded wait timeout -> shadow_state -> later detached-ready`.
- [x] 1.3 Preserve the relationship to `refactor-50`, `refactor-51`, and `refactor-52`, and keep
      UI/transport investigation out of scope for this bundle.

## 2. Design

- [x] 2.1 Define the boundary between waiting/parser-base residuals and the new
      `parse_exec/exact_ready_snapshot_assembly/program_lowering` residual.
- [x] 2.2 Decide whether the runtime fix belongs in parser-edit/reuse-plan continuity, detached-ready
      wakeup/consumption, fallback suppression, or truthful non-exact terminal reason; do not satisfy
      the change by widening budgets.
- [x] 2.3 Define per-cycle observability needed to prove full rebuild, reuse-plan miss, terminal
      semantic path, and final same-family lifecycle.

## 3. Implementation

- [x] 3.1 Add or adjust backend regression coverage for a same-version `didSave` producer that times
      out at `program_lowering`, exports `program_lowering_reuse_outcome=full_rebuild`, and would
      otherwise publish heavy follow-up through `shadow_state` before same-family detached-ready.
- [x] 3.2 Implement the root fix so still-current same-family `didSave` producers either avoid the
      full rebuild, reach detached diagnostics-ready before fallback, or export a truthful terminal
      non-exact reason.
- [x] 3.3 Extend the shadow fallback guard so it covers the rebuild-dominated `parse_exec` contour
      without blocking truthful supersession, cancellation, failure, or continuity-loss paths.
- [x] 3.4 Preserve diagnostics-save timeline and incident-bundle projection fields for
      program-lowering reuse outcome, rebuilt/reused units, reuse-plan hit flags, bounded-wait winner,
      terminal semantic path, and final producer lifecycle.
- [x] 3.5 Update the representative `examples/conf_big` live gate so the 2026-04-24T10:50:21Z bundle
      contour fails until the program-lowering rebuild/shadow fallback is repaired or truthfully
      explained.
- [x] 3.6 Add coverage that bounded-wait expiry plus
      `program_lowering_reuse_outcome=full_rebuild` cannot be reported as a truthful terminal reason
      while final same-family lifecycle later proves detached diagnostics-ready or full
      materialization.

## 4. Validation

- [x] 4.1 Run targeted backend diagnostics-save/program-lowering reuse and fallback regressions.
- [x] 4.2 Run representative live validation on `examples/conf_big` and record whether all captured
      cycles stay on `detached_ready_artifacts` or export truthful non-exact terminal reasons.
- [x] 4.3 Run cache-disabled or cold-cache controls if the implementation changes parser-edit or
      program-lowering reuse behavior.
- [x] 4.4 Run `cargo check --workspace --all-targets` and
      `cargo clippy --workspace --all-targets -- -D warnings` if production code changes.
- [x] 4.5 Run
      `openspec validate refactor-53-didsave-program-lowering-rebuild-shadow-fallback-bounding --strict --no-interactive`.

## 5. Evidence

- [x] Targeted regressions:
      `cargo test -p bsl-backend --bin bsl-lsp-server p53_diagnostics_save_timeline -- --nocapture`
      passed.
- [x] Parser-base continuity regression:
      `cargo test -p bsl-backend --bin bsl-lsp-server p52_diagnostics_save_timeline_exports_continuity_loss_for_started_parser_base_timeout -- --nocapture`
      passed.
- [x] Compile/lint:
      `cargo check --workspace --all-targets` passed;
      `cargo clippy --workspace --all-targets -- -D warnings` passed.
- [x] OpenSpec validation:
      `openspec validate refactor-53-didsave-program-lowering-rebuild-shadow-fallback-bounding --strict --no-interactive`
      passed after implementation updates.
- [x] Cache/cold controls: not applicable; this change does not alter parser-edit capture or
      program-lowering reuse-plan construction.
- [x] Representative live validation:
      `cargo test -p bsl-backend --bin bsl-lsp-server p56_real_conf_big_diagnostics_representative_save_followup_bundle_live -- --nocapture`
      passed in 462.08s and exported
      `backend/tests/perf/reports/refactor-49-save-followup-same-version-ready-snapshot-rebuild-bounding-real-conf-big-diagnostics-representative-save-followup-bundle-live.json`.
- [x] Broad diagnostics-save sweep:
      `cargo test -p bsl-backend --bin bsl-lsp-server diagnostics_save_timeline -- --nocapture`
      passed after the residual p31/p6/p7 fixes: 54 passed, 0 failed.
