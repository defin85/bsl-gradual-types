# lsp-completion-timeline v16

## 16.0.0

- Bumps public response envelope to `version=19`.
- Preserves all `v15` authoritative timing fields and adds bounded
  pre-dispatch adapter ingress detail inside `server_edge_details`:
  `adapter_read_at_ms` and `adapter_to_dispatch_wait_ms`.
- Keeps legacy `transport_received_at_ms` semantics unchanged so existing
  consumers can degrade gracefully while newer tooling isolates pre-dispatch
  adapter backlog from post-dispatch server ingress waits.

Migration note: timeline consumers must switch to `v16` and expect
`response.version=19`. Tooling that validates or documents the
server-generated payload must read optional `adapter_read_at_ms` and
`adapter_to_dispatch_wait_ms` fields inside `server_edge_details` and degrade
gracefully for `v18` payloads where the adapter ingress pre-dispatch split is
unavailable by design.
