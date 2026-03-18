# lsp-completion-timeline v5

## 5.0.0

- Bumps public response envelope to `version=3`.
- Preserves existing authoritative trace fields and adds optional bounded
  `server_edge_details` for server-edge completion diagnostics:
  - `transport_received_at_ms`
  - `handler_entered_at_ms`
  - `response_sent_at_ms`
  - optional `cancel_observed_at_ms`
  - `transport_to_handler_wait_ms`
  - `server_handler_exec_ms`
  - optional `cancel_observed_after_handler_enter_ms`

Migration note: timeline consumers must switch to `v5` and expect `response.version=3`.
If tooling reads server-edge completion diagnostics, it must use the bounded
`server_edge_details` object when present and remain backward compatible with older
payloads that only expose `response.version=2` without these fields.
