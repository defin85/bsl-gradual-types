## 1. Contract

- [x] 1.1 Add a `bsl-intellisense-v2` requirement for started same-version `didSave` exact
      producers to bound `parser_base_recovery` through detached diagnostics-ready publication or a
      truthful terminal producer reason.
- [x] 1.2 Add representative acceptance that fails on
      `started -> parser_base_recovery timeout -> shadow_state` when the save family remains
      current and no truthful terminal producer reason exists.
- [x] 1.3 Preserve the relationship to `refactor-51`, `refactor-43`, and `refactor-44`, and keep
      UI/transport investigation out of scope for this bundle.

## 2. Design

- [x] 2.1 Define the lifecycle boundary from `started` through `parser_base_recovery` to detached
      diagnostics-ready publication, full materialization, supersession, cancellation, failure, or
      continuity loss.
- [x] 2.2 Decide whether the runtime fix belongs in parser-base reuse/proof, detached-ready wake
      continuation, producer lifecycle tracking, or a truthful fallback reason; do not satisfy the
      change by widening budgets.
- [x] 2.3 Define per-cycle observability that records lifecycle at timeout and final lifecycle
      after timeout/fallback for the same save-family producer.

## 3. Implementation

- [x] 3.1 Add or adjust backend regression coverage for a same-version `didSave` producer that is
      already `started`, times out at `parser_base_recovery`, and would otherwise fall back through
      `shadow_state`.
- [x] 3.2 Implement the root fix so the still-current same-family producer either reaches detached
      diagnostics-ready before fallback or exports a truthful terminal producer reason.
- [x] 3.3 Update diagnostics-save timeline and incident-bundle projection with final same-family
      lifecycle evidence after timeout/fallback.
- [x] 3.4 Update the representative `examples/conf_big` live gate so the new bundle contour fails
      until the parser-base-to-detached-ready handoff is repaired.

## 4. Validation

- [x] 4.1 Run targeted backend diagnostics-save lifecycle/parser-base tests.
- [x] 4.2 Run representative live validation on `examples/conf_big` and record whether the terminal
      path is detached diagnostics-ready or a truthful non-exact producer outcome.

      Evidence: the warm 2026-04-24 p56 representative `examples/conf_big` rerun passed and wrote
      `validation/refactor-52-real-conf-big-diagnostics-representative-save-followup-bundle-live.json`,
      and the cold-control repros identified two cache-independent blockers: branch-context
      telemetry could block on the ready-state lock, and direct same-version `didSave` producers
      could start before `didChange` post-handoff supplied ranged parser edits, forcing
      `program_lowering` to `full_rebuild` under cold/cache-disabled runs.

      Fixed evidence: `BSL_CACHE_DISABLE=1` p56 passed and wrote
      `validation/refactor-52-real-conf-big-diagnostics-representative-save-followup-bundle-cache-disabled-live.json`
      with max `followup_publish_elapsed_ms=1384`, max
      `followup_ready_snapshot_parse_exec_ms=184`, `semantic_query_dominates_parse_exec_count=4`,
      and `program_lowering_reuse_outcome=routine_body_reuse` in all cycles. Empty `BSL_CACHE_DIR`
      p56 passed and wrote
      `validation/refactor-52-real-conf-big-diagnostics-representative-save-followup-bundle-cold-cache-live.json`
      with max `followup_publish_elapsed_ms=1449`, max
      `followup_ready_snapshot_parse_exec_ms=193`, `semantic_query_dominates_parse_exec_count=4`,
      and `program_lowering_reuse_outcome=routine_body_reuse` in all cycles. Both fixed controls
      stayed on `detached_ready_artifacts`, with no `shadow_state` fallback and final lifecycle
      `detached_diagnostics_ready_published`.
- [x] 4.3 Run `cargo check --workspace --all-targets` and
      `cargo clippy --workspace --all-targets -- -D warnings` if production code changes.
- [x] 4.4 Run
      `openspec validate refactor-52-didsave-parser-base-recovery-detached-ready-bounding --strict --no-interactive`.
