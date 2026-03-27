# lsp-completion-timeline v13

## 13.0.0

- Bumps public response envelope to `version=16`.
- Preserves existing authoritative trace fields and extends bounded
  `turn_attribution` with turn-wait resolution telemetry:
  - keeps `turn_wait_outcome` as the canonical dispatcher resolution outcome;
  - adds absolute `turn_wait_entered_at_ms` and `turn_wait_resolved_at_ms` so
    incident bundles can distinguish “waiter never resolved” from “resolved but
    resumed late”;
  - adds optional `wake_after_turn_resolution_at_ms` so bundle consumers can
    see when the waiting completion task was actually woken after dispatcher
    resolution;
  - keeps existing `queued_completion_ahead` / `active_holder` snapshots as the
    holder-level context for `request_id` / `request_epoch`.

Migration note: timeline consumers must switch to `v13` and expect
`response.version=16`. Tooling that validates or documents the
server-generated payload must read the expanded `turn_attribution` field set
and degrade gracefully for `v15` payloads where exact turn-wait
entered/resolved/wake timestamps are unavailable by design.
