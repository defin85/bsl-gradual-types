## 1. Exact follow-up blocker attribution

- [ ] 1.1 Distinguish `apply_lag` before exact ready-artifact availability from post-parse /
      post-ready publish gating when same-version exact artifacts already exist.
- [ ] 1.2 Export the new bounded blocker state through diagnostics save timeline and incident
      bundle surfaces without regressing existing `parse_exec` / relief-valve attribution.

## 2. Apply-lag isolation for exact publish

- [ ] 2.1 Let `didSave` idle-heavy follow-up publish from exact same-version ready artifacts
      without waiting for writer-owned apply when runtime can prove matching current
      `(file_id, requested_version, text_hash)`.
- [ ] 2.2 Preserve fail-closed behavior when exact proof is absent, artifacts are stale, or a
      newer save cycle supersedes the current one.

## 3. Regressions and live evidence

- [ ] 3.1 Add backend regressions for:
      exact follow-up publishing through `ready_artifacts` despite delayed apply on the mixed
      same-file path,
      truthful residual attribution when post-parse publish still cannot proceed,
      and non-regression of `skipped_apply_lag` when exact proof is absent.
- [ ] 3.2 Capture representative repo-local live evidence on `examples/conf_big` showing whether
      the mixed `didChange + didSave` path returns to `ready_artifacts`, or which bounded
      post-parse/apply-lag blocker remains.

## 4. Validation

- [ ] 4.1 Run targeted backend tests covering exact publish after delayed apply, residual blocker
      attribution, and the relevant `didSave` follow-up path.
- [ ] 4.2 Run `openspec validate refactor-26-diagnostics-save-exact-publish-apply-lag-isolation --strict --no-interactive`.

## 5. OpenSpec / Beads Sync

- [ ] 5.1 Keep Beads epic `bsl-gradual-types-lyr2` and children
      `bsl-gradual-types-lyr2.1` through `bsl-gradual-types-lyr2.4` aligned with the actual
      implementation status and dependency graph of this change.
