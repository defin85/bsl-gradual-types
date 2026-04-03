# lsp-completion-timeline v18

## 18.0.0

- Bumps public response envelope to `version=21`.
- Preserves all `v20` authoritative ingress and grouped query-body semantics.
- Adds optional flush-aware post-handler server egress split via
  `response_flush_completed_at_ms` and `response_ready_to_flush_wait_ms`.
- Keeps `response_sent_at_ms` semantics stable as the handler-local
  response-ready boundary; it is not reinterpreted as transport flush
  completion.

Migration note: timeline consumers must switch to `v18` and expect
`response.version=21`. Tooling that validates, documents, or summarizes the
server-generated payload must treat `response_ready_to_flush_wait_ms` as the
bounded server-only delay between handler-ready and actual transport flush, and
must degrade explicitly for `v20` payloads where flush-aware post-handler
egress split is unavailable by design.
