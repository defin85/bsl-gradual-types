## 1. Contract

- [x] 1.1 Add a `bsl-intellisense-v2` requirement that same-version `didSave` exact producers MUST
      use a dedicated save-critical admission contract through detached diagnostics-ready
      publication.
- [x] 1.2 Add representative acceptance that fails if a still-current same-version save family
      still terminates through waiting-phase `shadow_state` fallback before detached-ready publish.
- [x] 1.3 Import the `refactor-50` fail gate so representative validation also fails when
      `followup_semantic_path=shadow_state`, `followup_ready_snapshot_timeout_phase=waiting`, and
      query-dominated semantic fallback appear before later same-family exact readiness.

## 2. Design

- [x] 2.1 Describe the producer identity and lifecycle keyed to
      `(file_id, requested_version, text_hash, save_cycle_sequence)`, or a semantically equivalent
      save-family identity.
- [x] 2.2 Describe the dedicated save-critical admission lane and CPU-budget tier, and why this
      boundary must stay orthogonal to generic interactive and background work classes.
- [x] 2.3 Describe truthful fallback behavior when newer revision/save-cycle supersession or lost
      continuity proof makes the producer no longer safe to prefer.
- [x] 2.4 Describe why consumer-side `shadow_state` suppression or `generic_pipeline` fallthrough
      alone does not satisfy the inherited `refactor-50` gate without producer-owned admission and
      lifecycle evidence.

## 3. Implementation

- [x] 3.1 Add a dedicated same-version `didSave` exact-producer admission lane and CPU-budget tier
      in the runtime policy layer.
- [x] 3.2 Rework same-version `didSave` scheduling around producer ownership and lifecycle so the
      bounded contract ends at detached diagnostics-ready publication instead of depending on a
      mutable worker promotion path.
- [x] 3.3 Update heavy follow-up waiting, wakeup, and observability to consume producer lifecycle
      events truthfully and preserve fail-closed semantics.
- [x] 3.4 Update regressions and representative live gate expectations so waiting-only
      `shadow_state` terminal publish is no longer accepted for still-current save families.
- [x] 3.5 Update the representative `examples/conf_big` gate to fail on the full inherited
      `refactor-50` contour: waiting-phase timeout, `shadow_state` terminal path,
      query-dominated semantic fallback, and later same-family exact readiness.

## 4. Validation

- [x] 4.1 Run targeted backend/runtime/diagnostics-save regressions for the dedicated
      save-critical producer path.
- [x] 4.2 Run representative live/perf validation for the same-version `didSave` exact-producer
      gate on `examples/conf_big`.
- [x] 4.3 Run `openspec validate refactor-51-didsave-exact-producer-lane-bounding --strict --no-interactive`.
