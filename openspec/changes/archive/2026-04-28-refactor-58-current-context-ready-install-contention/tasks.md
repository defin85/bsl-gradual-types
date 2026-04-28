## 1. Contract and Evidence

- [x] 1.1 Record the incident evidence from
      `/home/egor/code/temp/bsl-observability-incident-2026-04-26T21-01-14Z`
      and keep it linked from the change.
- [x] 1.2 Add a `bsl-intellisense-v2` requirement for bounded,
      attributable current-context and didSave ready-install contention.
- [x] 1.3 State explicit non-goals for `refactor-57`, `refactor-50`,
      completion/UI dispatch, and pure budget widening.

## 2. Instrumentation

- [x] 2.1 Add or extend per-request `bsl.getCurrentContext` timeline
      observability with route, generation/version, broker role, wait results,
      elapsed times, supersession/budget outcome, and final status.
- [x] 2.2 Project current-context request evidence into incident bundles as a
      first-class section, correlated with completion and didSave windows where
      available.
- [x] 2.3 Split didSave follow-up `ready_install`, `snapshot_with_deps`, and
      `wait_for_file_version` residuals into stable bounded attribution buckets.
- [x] 2.4 Add an explicit `unclassified_readiness_residual` or equivalent
      failure-visible bucket so missing attribution is detectable.

## 3. Runtime Behavior

- [x] 3.1 Audit `bsl.getCurrentContext` ready-snapshot,
      latest-only-stabilization, and parse-broker paths for stale work,
      follower waits, and budget behavior.
- [x] 3.2 Bound or short-circuit stale current-context work when a newer
      generation supersedes it, while preserving correct latest results.
- [x] 3.3 Audit didSave follow-up readiness/install waits after `parse_exec`
      completion and remove, bound, or classify seconds-scale waits.
- [x] 3.4 Preserve completion isolation under concurrent current-context and
      diagnostics load.

## 4. Tests

- [x] 4.1 Add focused current-context mixed-load coverage proving equivalent
      same-generation bursts share bounded work and stale generations report
      supersession instead of accumulating opaque seconds-scale waits.
- [x] 4.2 Add didSave follow-up regression coverage for a fast `parse_exec` with
      slow readiness/install wait, asserting explicit blocker attribution.
- [x] 4.3 Add incident-bundle projection coverage for the new current-context
      section and readiness attribution buckets.
- [x] 4.4 Add negative/guard coverage proving budget widening alone cannot
      satisfy acceptance when residual readiness waits remain unclassified.

## 5. Validation

- [x] 5.1 Run the focused backend/LSP tests added or touched by this change.
- [x] 5.2 Run the relevant runtime/facade tests for
      `snapshot_with_deps` / `wait_for_file_version` attribution if those paths
      are changed.
- [x] 5.3 Capture a fresh representative incident bundle or equivalent live
      observability snapshot and verify:
      - `intellisense_v2_observability_contract_violation_total` is absent or
        `0`;
      - invalid saturation metric violations are absent or `0`;
      - current-context requests appear as first-class request evidence;
      - didSave follow-up `ready_install` / `snapshot_with_deps` seconds-scale
        residuals are removed or explicitly classified.
- [x] 5.4 Run `cargo check --workspace --all-targets`.
- [x] 5.5 Run `cargo clippy --workspace --all-targets -- -D warnings` if
      production Rust changes are made.
- [x] 5.6 Run
      `openspec validate refactor-58-current-context-ready-install-contention --strict --no-interactive`.
