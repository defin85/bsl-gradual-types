# lsp-completion-timeline v20

## 20.0.0

- Bumps public response envelope to `version=23`.
- Preserves all `v22` shipped compatibility milestones and derived waits.
- Adds optional truthful encode-start milestone
  `response_output_encode_started_at_ms`.
- Re-defines `response_output_write_started_at_ms` as the literal first write
  boundary in the outbound transport path.
- Keeps `response_output_encode_completed_at_ms` as the encode completion
  boundary and `response_flush_completed_at_ms` as the flush completion
  boundary.
- Re-anchors derived waits so that they describe disjoint intervals:
  `response_output_queue_wait_ms = encode_started - enqueue_completed`,
  `response_output_encode_exec_ms = encode_completed - encode_started`, and
  `response_output_write_and_flush_exec_ms = flush_completed - write_started`.
- Keeps `response_ready_to_flush_wait_ms` only as umbrella compatibility
  evidence across the full post-handler path.

Migration note: timeline consumers must switch to `v20` and expect
`response.version=23`. Tooling that validates, documents, or summarizes the
server-generated payload must read `response_output_encode_started_at_ms` when
it is available, interpret `response_output_write_started_at_ms` as literal
first write start, keep `response_ready_to_flush_wait_ms` only as umbrella
compatibility evidence, and degrade explicitly for `v22` payloads where the
truthful encode-start vs write-start boundary is unavailable by design.
