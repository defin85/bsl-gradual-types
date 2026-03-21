# lsp-completion-timeline v7

## 7.0.0

- Bumps public response envelope to `version=10`.
- Preserves existing authoritative trace fields and extends the bounded
  `server_edge_details` contract with dispatch-to-request-context attribution:
  - adds `transport_received_at_ms_provenance`;
  - adds optional `jsonrpc_dispatch_received_at_ms`;
  - adds optional `dispatch_to_request_context_wait_ms`.

Migration note: timeline consumers must switch to `v7` and expect
`response.version=10`. Tooling that validates or documents the server-generated
payload must read the expanded `server_edge_details` field set and stop assuming
that `transport_received_at_ms` always means request-context call entry.
