# lsp-completion-timeline v6

## 6.0.0

- Bumps public response envelope to `version=9`.
- Preserves existing authoritative trace fields and aligns the versioned contract
  baseline with the current bounded completion timeline payload:
  - extends `prepare_details` with bounded `progress`, runtime drilldown,
    `timeout_attribution` and `exact_wait` objects;
  - extends `turn_attribution` with bounded
    `dispatcher_resolution_latency_ms`;
  - extends `server_edge_details` with bounded pre-method and pre-service-scope
    attribution fields, including trustworthy provenance and the additive
    `service_future_created_at_ms` split.

Migration note: timeline consumers must switch to `v6` and expect
`response.version=9`. Tooling that validates or documents the server-generated
payload must read the new bounded nested contract sections and the expanded
`server_edge_details` / `turn_attribution` field sets instead of assuming the
older `v5` / `response.version=3` surface.
