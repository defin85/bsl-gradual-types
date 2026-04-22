## 1. Contract

- [x] 1.1 Add a `bsl-intellisense-v2` requirement that same-file `didChange`
      current-revision handoff for `(file_id, version)` progresses on a minimal ingress fast lane
      before full handler/background stages.
- [x] 1.2 Define truthful publication semantics for the fast lane:
      same-file ingress token is still published only after real handoff registration, not from
      earlier dispatcher/barrier bookkeeping.
- [x] 1.3 Define representative mixed-load acceptance that fails if post-`didChange` completion
      still spends seconds-scale wait in `completion_barrier_wait_ms` or
      `same_file_ingress_token_wait_ms` after same-file ingress was already observed.

## 2. Design

- [x] 2.1 Describe the minimal didChange fast-lane responsibilities:
      accepted-text derivation, shadow/latest-received update, current-revision handoff
      registration, and token publication.
- [x] 2.2 Describe how downstream `lsp_did_change` work reuses that authoritative revision without
      double-applying `SetFile`, stale overwrites, or stronger-than-truth readiness claims.
- [x] 2.3 Describe latest-wins, out-of-order, and supersession semantics for same-file revisions
      on the fast lane.
- [x] 2.4 Describe the representative live/perf evidence and the worst-outlier correlation slice
      that proves the new handoff boundary is actually bounded.

## 3. Implementation

- [x] 3.1 Introduce the minimal same-file `didChange` handoff fast lane or an equivalent
      starvation-safe mechanism that registers current revision before delayed full-handler work.
- [x] 3.2 Keep the existing downstream `lsp_did_change` parse-snapshot / diagnostics /
      observability work correct on top of the already-registered authoritative revision.
- [x] 3.3 Add regressions for same-file `didChange` -> completion ordering, superseded/out-of-order
      revisions, and truthful token publication.
- [ ] 3.4 Refresh representative mixed-load evidence on `examples/conf_big` showing that
      post-edit completion no longer spends seconds-scale latency in
      `completion_barrier_wait_ms` / `same_file_ingress_token_wait_ms`.

## 4. Validation

- [x] 4.1 Run targeted backend/runtime/transport regressions for the new didChange handoff fast
      lane and its same-file ordering semantics.
- [ ] 4.2 Run representative live/perf validation for the new same-file mixed-load gate.
- [x] 4.3 Run `openspec validate refactor-48-didchange-current-revision-handoff-fast-lane --strict --no-interactive`.
