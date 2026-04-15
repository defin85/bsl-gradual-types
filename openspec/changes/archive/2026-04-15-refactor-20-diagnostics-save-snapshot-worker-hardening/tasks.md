## 1. Implementation

- [x] 1.1 Add shared control state for background ready-snapshot workers so supersession,
      materialization notification, and exact same-version promotion are tracked independently of
      outer async task abort.
- [x] 1.2 Make debounce and parse-build stages honor cooperative supersession, including a
      cancellation-aware incremental/full parse path for the parser-coordinator snapshot worker.
- [x] 1.3 Teach `didSave` heavy follow-up to promote an existing exact same-version worker into the
      `did_save_followup` lane, including workers that already crossed debounce and are queued in
      runtime admission, and boundedly wait for materialization before `shadow_state` fallback,
      without spawning a duplicate `didSave` worker for identical text/version.
- [x] 1.4 Teach `bsl.getCurrentContext` to consume or briefly await an equivalent exact
      same-version snapshot worker before launching an independent `parser_coordinator` parse,
      while preserving latest-generation supersession semantics.
- [x] 1.5 Keep snapshot-backed `SetFileWithSnapshot` install on the background writer path;
      promotion must change materialization scheduling, not current-revision handoff semantics.

## 2. Validation

- [x] 2.1 Add regressions proving superseded `didChange` snapshot workers stop cooperatively and do
      not remain abort-only parser contention after a newer revision arrives.
- [x] 2.2 Add regressions proving `didSave` can promote an in-flight exact same-version
      `didChange` worker, including an already-queued worker, and publish richer follow-up in the
      same save cycle without starting a duplicate `didSave` snapshot worker.
- [x] 2.3 Add regressions proving `bsl.getCurrentContext` reuses an equivalent same-version
      snapshot task before independent parse and still honors newest-generation-wins semantics
      under cursor bursts.
- [x] 2.4 Capture representative evidence on the `conf_big`-like mixed-load profile showing that
      the target `didSave` cycle no longer remains stuck in pure pending-publish / exact-wait
      starvation, and that any residual exact-worker miss falls through truthful
      `shadow_state`/`semantic_work` attribution instead of hidden queue starvation.
- [x] 2.5 Run `openspec validate refactor-20-diagnostics-save-snapshot-worker-hardening --strict --no-interactive`.
