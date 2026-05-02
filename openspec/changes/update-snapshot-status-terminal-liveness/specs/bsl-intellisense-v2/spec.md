## ADDED Requirements

### Requirement: Snapshot status building states have terminal liveness (MUST)
The LSP server SHALL ensure that any snapshot status reported as `state=building` for a requested document revision has an authoritative terminal lifecycle.

For a given file and requested revision, after the matching snapshot worker is materialized, cancelled, superseded, failed, times out at an exact-index readiness boundary, or is removed from the in-flight task registry, the next authoritative snapshot status notification or `bsl/getSnapshotStatus` response SHALL report the post-transition state. It MUST NOT leave the last visible state indefinitely at the old `building` revision.

Any code path that removes or aborts a matching snapshot worker outside the worker future SHALL refresh or cache snapshot status after the task is removed, because the aborted worker might not reach its normal final cleanup path.

Terminal or post-transition states SHALL include one of:

- `ready` with `exact=true` when canonical same-revision artifacts are ready;
- `failed` when the same-revision build reached an explicit failure;
- `shadow_only`, `stale`, or `idle` when canonical same-revision artifacts are unavailable and no matching worker remains;
- a newer requested revision state when the old revision was superseded.

When a same-revision explicit failure and current editor shadow are both present for the current requested revision, `failed` SHALL take precedence over `shadow_only`, `stale`, and `idle` unless the requested revision has already been superseded by a newer revision.

Notification coalescing MAY suppress age-only or phase-only churn, but it MUST NOT suppress semantic lifecycle transitions from `building` to a terminal or superseding state.

`bsl/getSnapshotStatus` SHALL recompute from authoritative runtime state and SHALL NOT return a stale cached `building` state after the server can observe that the matching worker is no longer current.

#### Scenario: Building revision becomes ready
- **GIVEN** the server has reported snapshot status `state=building` for requested revision `V`
- **AND** canonical same-revision ready parse, exact index, and completion-head artifacts become ready
- **WHEN** the worker cleanup completes or the client requests `bsl/getSnapshotStatus`
- **THEN** the server reports `state=ready`
- **AND** `requestedVersion=V`
- **AND** `readyVersion=V`
- **AND** `exact=true`
- **AND** the previous `building` state is not retained as the active status

#### Scenario: Building revision is superseded by a newer revision
- **GIVEN** the server has reported snapshot status `state=building` for requested revision `V`
- **AND** a newer document revision `V+1` becomes the latest requested revision
- **WHEN** the server refreshes snapshot status
- **THEN** the active status reflects revision `V+1` or an explicit non-current task state
- **AND** the UI is not left showing `building requested=V` as the current active-document truth

#### Scenario: Coalescing does not hide terminal status
- **GIVEN** the latest cached status is `state=building`
- **AND** the newly computed status differs only by lifecycle-significant fields such as state, exactness, requested revision, ready revision, task state, or artifact readiness
- **WHEN** live snapshot notifications are coalesced
- **THEN** the server emits or stores the new lifecycle-significant status
- **AND** does not classify the transition as phase-only or age-only churn

#### Scenario: Manual refresh repairs missed notification
- **GIVEN** a live notification was missed or delayed while the active document still shows `state=building`
- **AND** the backend can observe that the matching worker has exited or the artifacts have advanced
- **WHEN** the client sends `bsl/getSnapshotStatus`
- **THEN** the response reflects the current authoritative state
- **AND** the server-side latest snapshot status cache is updated to that state

#### Scenario: External cancellation refreshes after task removal
- **GIVEN** the server has reported snapshot status `state=building` for requested revision `V`
- **WHEN** an external cancellation path removes or aborts the matching background snapshot worker
- **THEN** the server refreshes or stores snapshot status after the task is removed
- **AND** the active status is no longer `building` for the removed worker

#### Scenario: Explicit failure wins over shadow-only
- **GIVEN** the active document shadow is current for requested revision `V`
- **AND** the matching snapshot worker records an explicit same-revision build failure for `V`
- **WHEN** the worker exits or the client requests `bsl/getSnapshotStatus`
- **THEN** the server reports `state=failed`
- **AND** does not report `shadow_only`, `stale`, or `idle` for that same failed revision
