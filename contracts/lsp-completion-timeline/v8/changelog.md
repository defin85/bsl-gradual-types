# lsp-completion-timeline v8

## 8.0.0

- Bumps public response envelope to `version=11`.
- Preserves existing authoritative trace fields and extends the bounded
  `server_edge_details` contract with service-future first-poll / first-wake
  attribution:
  - adds optional `service_future_first_poll_entered_at_ms`;
  - adds optional `service_future_to_first_poll_wait_ms`;
  - adds optional bounded `service_future_first_poll_outcome`;
  - adds optional `service_future_first_wake_scheduled_at_ms`;
  - adds optional `first_poll_to_first_wake_wait_ms`.

Migration note: timeline consumers must switch to `v8` and expect
`response.version=11`. Tooling that validates or documents the
server-generated payload must read the expanded `server_edge_details` field set
and stop assuming that the pre-service-scope segment is the smallest
authoritative split available.
