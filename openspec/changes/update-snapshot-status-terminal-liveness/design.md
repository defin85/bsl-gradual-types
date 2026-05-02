## Context
`add-snapshot-readiness-diagnostics-view` added bounded snapshot readiness detail to `bsl/getSnapshotStatus`, `bsl/snapshotStatus`, the status bar tooltip, and the Observability Tree View. The incident after that change exposed a narrower runtime/liveness gap: the UI showed a truthful last notification, but the last notification was no longer the current runtime truth.

Evidence from the 2026-05-01 live session:

- VS Code output ended with `state=building requested=v38 ready=v36 exact=false task=in_flight_same_revision`.
- Rust LSP logs later showed `type_index_precompute_exact_stored` for `requested_version=38`.
- Rust LSP logs also showed diagnostics publication for `expected_version=38`.
- No terminal `ready`, `failed`, `stale`, `shadow_only`, or superseding status notification for `v38` reached the client.

## Goals
- Make `building` a live state with an observable terminal lifecycle.
- Keep status truth server-driven; clients must not infer readiness from other surfaces.
- Preserve bounded notification coalescing while treating lifecycle transitions as meaningful.
- Ensure manual refresh via `bsl/getSnapshotStatus` can repair missed notification state.
- Add tests that fail on indefinite stale `building` status after worker progress or cleanup.

## Non-Goals
- Do not solve `ТаблЗнач.` children completion in this change.
- Do not alter exact semantic artifact correctness or fallback policy.
- Do not introduce unbounded polling loops in the VS Code extension.
- Do not conflate diagnostics publication with exact readiness; diagnostics can be evidence, not the source of snapshot truth.

## Design Notes

### 1. Terminal refresh is part of worker lifecycle
Every background parse snapshot apply worker path that exits, retargets, cancels, fails, reaches exact-index deadline, materializes, or is removed from `background_parse_snapshot_apply_tasks_v2` must refresh snapshot status after the authoritative store/task state changes.

The final refresh must see post-cleanup state. If the task has been removed and exact/ready artifacts are present, the status should be `ready`. If only shadow is current and canonical artifacts are absent, the status should be `shadow_only`. If the latest requested version has moved on, the visible status should reflect the newer requested revision or a superseded/non-current task state.

### 2. Coalescing cannot hide semantic lifecycle transitions
The notification coalescer may suppress age-only and phase-only churn, but it must not suppress:

- `building` to `ready`;
- `building` to `failed`;
- `building` to `shadow_only` or `stale`;
- `building requested=vN` to a newer requested revision;
- changes in artifact readiness that change operator action, such as exact/completion-head moving from `building` or `missing` to `ready`.

### 3. Request path is a repair source
`bsl/getSnapshotStatus` must recompute from authoritative runtime state and update the server-side latest status cache. A manual refresh should not return the previous cached `building` if the worker has already exited or the artifacts have advanced.

### 4. Failure and external cancellation have explicit precedence
Same-revision explicit failure is a terminal state. If the current requested revision has a recorded build failure and has not been superseded by a newer requested revision, snapshot status must report `failed` instead of downgrading the same situation to `shadow_only`, `stale`, or `idle`.

External cancellation paths that remove or abort a worker outside the worker future must refresh snapshot status after task removal. They cannot rely on the aborted worker to run the normal final cleanup refresh.

### 5. Incident evidence remains explicit
The implementation should add a regression that can express the incident shape without depending on wall-clock flakiness:

- emit or cache a `building requested=vN ready=vN-k` state;
- advance the backend to exact/diagnostics-ready or cleanup state for `vN`;
- assert that the next notification or `getSnapshotStatus` response is terminal or superseded, not the original stale `building`.

## Risks
- Over-emitting notifications could reintroduce UI/log churn; tests should keep phase-only coalescing green.
- Treating diagnostics publication as exact readiness would be incorrect; diagnostics should only help construct a deterministic test setup or evidence.
- If worker cleanup currently runs after side work, terminal status can lag; the change should distinguish legitimate short side-work lag from indefinite stale state.
