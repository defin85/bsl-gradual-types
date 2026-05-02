## 1. Backend Lifecycle Contract
- [x] 1.1 Trace every background parse snapshot apply terminal path and identify where `refresh_snapshot_status_v2` runs relative to task removal and artifact publication.
- [x] 1.2 Ensure materialized, cancelled, superseded, exact-index-deadline, latest-version-mismatch, failed, and retargeted terminal paths emit or cache post-transition snapshot status, including external abort/remove paths that cannot rely on worker-final cleanup.
- [x] 1.3 Tighten snapshot-status coalescing so semantic lifecycle transitions are never suppressed as phase-only churn.
- [x] 1.4 Ensure `bsl/getSnapshotStatus` recomputes and updates cache from authoritative state even when the latest live notification was stale.
- [x] 1.5 Ensure same-revision explicit failure has priority over `shadow_only`, `stale`, and `idle` when computing terminal status for the current requested revision.

## 2. Regression Coverage
- [x] 2.1 Add backend tests for `building -> ready` notification after worker cleanup.
- [x] 2.2 Add backend tests for `building -> shadow_only/stale/failed` terminal transitions when exact artifacts do not become ready.
- [x] 2.3 Add backend tests for superseded newer-version transitions so old `building requested=vN` does not remain active after `vN+1`.
- [x] 2.4 Preserve existing phase-only/age-only coalescing tests.
- [x] 2.5 Add an incident-shape test for `requested=v38 ready=v36` advancing to a terminal or superseded status.
- [x] 2.6 Add backend tests for explicit failure precedence and external cancellation/abort refreshing status after task removal.

## 3. Observability Evidence
- [x] 3.1 Capture a before/after snapshot-status transition trace or incident bundle showing the stale-building failure no longer reproduces.
- [x] 3.2 Document that completion/member-access failures are intentionally deferred to `update-local-variable-member-completion-children`.

## 4. Validation
- [x] 4.1 Run `openspec validate update-snapshot-status-terminal-liveness --strict --no-interactive`.
- [x] 4.2 Run targeted backend snapshot-status tests.
- [x] 4.3 Run any touched VS Code snapshot-status tests if client logic changes.
- [x] 4.4 Run formatting/checks required by touched Rust/TypeScript paths.
