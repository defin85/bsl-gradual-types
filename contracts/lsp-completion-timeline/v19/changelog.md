# lsp-completion-timeline v19

## 19.0.0

- Bumps public response envelope to `version=22`.
- Preserves all `v21` authoritative ingress, grouped query-body, and coarse
  flush-aware post-handler semantics.
- Adds optional finer output-egress milestones via
  `response_output_enqueue_completed_at_ms`,
  `response_output_write_started_at_ms`, and
  `response_output_encode_completed_at_ms`.
- Adds optional derived server-only waits via
  `response_ready_to_output_enqueue_wait_ms`,
  `response_output_queue_wait_ms`, `response_output_encode_exec_ms`, and
  `response_output_write_and_flush_exec_ms`.
- Keeps `response_sent_at_ms` semantics stable as the handler-local
  response-ready boundary; `response_flush_completed_at_ms` remains the flush
  completion boundary.

Migration note: timeline consumers must switch to `v19` and expect
`response.version=22`. Tooling that validates, documents, or summarizes the
server-generated payload must read the finer `v22` output-egress split when it
is available, keep `response_ready_to_flush_wait_ms` only as umbrella
compatibility evidence, and degrade explicitly for `v21` payloads where finer
output-egress split is unavailable by design.
