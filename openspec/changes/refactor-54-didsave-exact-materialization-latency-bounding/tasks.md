## 1. Contract

- [x] 1.1 Add a `bsl-intellisense-v2` requirement for the post-refactor-53 residual where
      `detached_ready_artifacts` is the terminal path but exact materialization arrives after
      bounded wait and relief-valve timeouts.
- [x] 1.2 Add a `bsl-intellisense-v2` requirement for slow `save_fastlane` syntax-only first
      publish, so a later successful heavy follow-up cannot hide multi-second first-publish
      latency.
- [x] 1.3 Record the 2026-04-24T14:22:42Z incident evidence and keep UI/transport and terminal
      `shadow_state` fallback out of scope.

## 2. Design

- [x] 2.1 Define the boundary between refactor-53 terminal-path correctness and the new detached
      ready-but-too-late materialization residual.
- [x] 2.2 Define implementation options for first-publish syntax latency and exact
      `program_lowering` full rebuild latency without prescribing an unverified cache mechanism.
- [x] 2.3 Define per-cycle observability required to prove first-publish syntax blockers, exact
      phase attribution, reuse/rebuild counts, terminal semantic path, and final lifecycle.

## 3. Implementation

- [ ] 3.1 Inspect the `save_fastlane` syntax-only publish path and identify why the representative
      cycle can spend `3397ms` in `syntax_diagnostics_query_ms` while exact producer
      `parser_base_recovery` is also active.
- [ ] 3.2 Add targeted regression coverage for slow first-publish syntax query attribution and the
      requirement that a later detached-ready follow-up does not hide first-publish latency.
- [ ] 3.3 Inspect the same-version exact producer path that led to
      `program_lowering_reuse_outcome=full_rebuild`, `2088` rebuilt units, and `0` reused units in
      the fresh bundle.
- [ ] 3.4 Implement the root fix so a still-current representative `didSave` producer either avoids
      the full rebuild, reaches detached diagnostics-ready within the existing envelope, or exports
      a truthful non-exact terminal reason independent of bounded-wait expiry and full-rebuild reuse
      miss.
- [ ] 3.5 Preserve or extend diagnostics-save timeline and incident-bundle projections for
      first-publish syntax timing, parser-base and program-lowering phase timings,
      `program_lowering` reuse/rebuild evidence, bounded wait and relief-valve outcomes, terminal
      semantic path, and final lifecycle.
- [ ] 3.6 Update the representative `examples/conf_big` live/perf gate so the
      2026-04-24T14:22:42Z contours fail until first-publish and detached-ready materialization
      latency are repaired or truthfully classified.

## 4. Validation

- [ ] 4.1 Run targeted backend diagnostics-save timeline regressions for first-publish syntax
      latency and detached-ready materialization latency.
- [ ] 4.2 Run targeted `program_lowering` reuse/rebuild regressions if the implementation changes
      reuse planning, parser-edit continuity, or ownership-based materialization.
- [ ] 4.3 Run representative live validation on `examples/conf_big` and record whether all captured
      save cycles satisfy first-publish and heavy-follow-up latency gates while preserving
      `detached_ready_artifacts`.
- [ ] 4.4 Run `cargo check --workspace --all-targets` and
      `cargo clippy --workspace --all-targets -- -D warnings` if production code changes.
- [x] 4.5 Run
      `openspec validate refactor-54-didsave-exact-materialization-latency-bounding --strict --no-interactive`.

## 5. Initial Evidence

- [x] Incident bundle:
      `/home/egor/code/temp/bsl-observability-incident-2026-04-24T14-22-42Z`.
- [x] Completion evidence: 6 captured completion traces; max completion `190ms` dominated by
      `collect`; `client_before_transport_write_wait_ms=1-2`; transport/admission/same-file
      ingress and response handoff are near zero.
- [x] Diagnostics-save trace 1: first publish `3397ms`, `syntax_diagnostics_query_ms=3397`,
      `parser_base_recovery=3926ms`, follow-up `577ms`, terminal
      `detached_ready_artifacts`.
- [x] Diagnostics-save trace 2: first publish `55ms`, heavy follow-up `4884ms`, bounded wait
      timeout `3502ms`, relief timeout `501ms`, `program_lowering=4230ms`,
      `program_lowering_reuse_outcome=full_rebuild`, `2088` rebuilt units, `0` reused units,
      terminal `detached_ready_artifacts`.
