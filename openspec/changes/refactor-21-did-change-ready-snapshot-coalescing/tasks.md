## 1. Runtime contract

- [x] 1.1 Replace same-file spawn-per-`didChange` ready-snapshot scheduling with a file-scoped
      latest-wins coalesced producer that can retarget newer exact revisions before wasting more
      parse/materialization work.
- [x] 1.2 Keep exact latest-wins semantics: obsolete intermediate revisions MUST NOT materialize or
      publish ready artifacts once a newer same-file target supersedes them.
- [x] 1.3 Update `didSave` heavy follow-up so bounded wait is attempted only for an exact
      still-current coalesced producer for `(file_id, requested_version, text_hash)` and falls back
      immediately when that producer was already retargeted away.

## 2. Observability and regressions

- [x] 2.1 Add low-cardinality observability for coalesced producer lifecycle so bundles can tell
      apart `retargeted_before_parse`, `retargeted_before_materialize`, exact wait success, and
      truthful fallback after timeout.
- [x] 2.2 Add regressions for same-file burst coalescing, including a case where older `didChange`
      revisions are coalesced away before parse starts and a case where a parsed older revision is
      skipped before materialization because a newer target already exists.
- [x] 2.3 Add `didSave` regressions proving the heavy follow-up waits only for an exact
      still-current producer and skips bounded waiting for coalesced-away revisions.
- [x] 2.4 Capture representative repo-local evidence showing reduced same-file worker churn and a
      higher share of exact `ready_artifacts` reuse on the same `conf_big` save cycle.

## 3. Validation

- [x] 3.1 Run targeted backend tests for new coalescing / `didSave` wait behavior.
- [x] 3.2 Run repo-local live evidence command(s) for the updated diagnostics-save / incident-bundle
      path.
- [x] 3.3 Run `openspec validate refactor-21-did-change-ready-snapshot-coalescing --strict --no-interactive`.
