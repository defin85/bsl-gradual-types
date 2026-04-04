# lsp-completion-timeline v21

## 21.0.0

- Bumps public response envelope to `version=24`.
- Preserves all `v23` additive egress fields and keeps
  `response_output_enqueue_completed_at_ms` only as a legacy writer-selection
  compatibility seam.
- Adds truthful send-side handoff milestones
  `response_output_handoff_started_at_ms` and
  `response_output_handoff_enqueued_at_ms`.
- Adds disjoint `v24` handoff waits:
  `response_ready_to_output_handoff_wait_ms`,
  `response_output_handoff_send_wait_ms`, and
  `response_output_handoff_to_writer_wait_ms`.
- Keeps `response_ready_to_output_enqueue_wait_ms` only as umbrella
  compatibility evidence across the full `response_sent -> writer_selection`
  interval.

Migration note: timeline consumers must switch to `v21` and expect
`response.version=24`. Tooling that validates, documents, or summarizes the
server-generated payload must read the new `response_output_handoff_*` fields
when they are available, interpret `response_output_enqueue_completed_at_ms` as
legacy writer-selection compatibility evidence rather than send-side enqueue
acceptance, keep `response_ready_to_output_enqueue_wait_ms` only as umbrella
compatibility evidence, and degrade explicitly for `v23` payloads where the
truthful pre-enqueue handoff split is unavailable by design.
