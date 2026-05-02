# Snapshot Terminal Liveness Evidence

Date: 2026-05-01

## Lifecycle Trace

The backend terminal path was traced across the snapshot readiness surfaces touched by this
change:

- `snapshot_status_for_uri_v2` and `bsl/getSnapshotStatus` both go through
  `upsert_snapshot_status_v2`, recompute authoritative state with
  `compute_snapshot_status_v2`, and update `latest_snapshot_status_v2`.
- Live notifications are emitted from `refresh_snapshot_status_v2`; coalescing ignores only
  phase, trigger, and worker age churn. State, requested version, ready version, exactness,
  task state, worker target/supersession, reason, and artifact readiness remain
  lifecycle-significant.
- Normal worker lifecycle already refreshes after task creation, phase changes,
  canonical ready publication, and final task removal.
- The external abort path `cancel_background_parse_snapshot_apply_v2` removed and aborted the
  task without a post-remove refresh. This change adds that refresh.
- Same-revision failures were recorded but could be masked by current shadow or stale ready
  artifacts. This change gives explicit same-revision failure priority over `shadow_only`,
  `stale`, and `idle` after no same-revision worker remains.

## Before / After Regressions

The deterministic regression coverage now exercises the stale-building shapes without relying on
wall-clock live incidents:

- `snapshot_status_live_notification_emits_ready_after_worker_cleanup`: `building -> ready`
  after worker removal and same-revision ready artifact publication.
- `snapshot_status_live_notification_emits_shadow_only_after_worker_cleanup_without_artifacts`:
  `building -> shadow_only` after worker removal without canonical artifacts.
- `snapshot_status_live_notification_emits_superseding_revision_for_old_building_worker`:
  `building requested=v38 -> requested=v39` with `in_flight_other_revision` and
  `supersededByVersion=39`.
- `snapshot_status_get_request_repairs_stale_building_cache_after_worker_removed`:
  manual `bsl/getSnapshotStatus` recomputes ready state and updates the cache after a stale
  building cache entry.
- `snapshot_status_request_reports_failed_before_shadow_only_for_same_revision_failure`:
  explicit same-revision failure wins over current shadow.
- `snapshot_status_request_reports_failed_before_stale_for_same_revision_failure`:
  explicit same-revision failure wins over stale ready artifacts.
- `snapshot_status_incident_shape_does_not_keep_stale_building_after_failure_terminal`:
  incident shape `building requested=v38 ready=v36` advances to terminal `failed` after task
  removal and failure recording.
- `snapshot_status_external_cancel_refreshes_after_task_removal`:
  external cancellation emits a post-remove terminal status instead of leaving the cache at
  `building`.
- `snapshot_status_live_notifications_coalesce_phase_only_building_transitions` remains green,
  preserving the age/phase-only coalescing contract while allowing terminal transitions.

## Deferred Scope

Completion/member-access behavior, including `ТаблЗнач.` children completion, is intentionally
deferred to `update-local-variable-member-completion-children`. This change only makes the
snapshot readiness signal terminal and repairable enough to interpret that follow-up accurately.

## Validation Commands

- `cargo test -p bsl-backend snapshot_status_live_notification_emits -- --nocapture`
- `cargo test -p bsl-backend snapshot_status_request_reports_failed_before -- --nocapture`
- `cargo test -p bsl-backend snapshot_status_get_request_repairs_stale_building_cache_after_worker_removed -- --nocapture`
- `cargo test -p bsl-backend snapshot_status_incident_shape_does_not_keep_stale_building_after_failure_terminal -- --nocapture`
- `cargo test -p bsl-backend snapshot_status_external_cancel_refreshes_after_task_removal -- --nocapture`
- `cargo test -p bsl-backend snapshot_status_live_notifications_coalesce_phase_only_building_transitions -- --nocapture`
