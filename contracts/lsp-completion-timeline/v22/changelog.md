# lsp-completion-timeline v22

## 22.0.0

- Bumps public response envelope to `version=25`.
- Preserves all `v24` truthful handoff and legacy writer-selection compatibility
  seams.
- Adds truthful completion pre-dispatch decomposition for transport reader wait,
  admission queue residence, scheduler `poll_ready()` wait, completion barrier
  ownership, same-file ingress-token gating, and residual post-ready delay.
- Adds bounded reader/backpressure and admission-state fields including
  `adapter_read_started_at_ms`, `adapter_parse_completed_at_ms`,
  `read_loop_wait_reason`, `read_loop_wait_ms`,
  `pending_completion_spillover_depth`, `pending_general_request_staged`,
  `admission_try_enqueue_at_ms`, `admission_lane`,
  `admission_lane_depth_before`, `admission_lane_depth_after`,
  `admission_enqueue_outcome`, `admission_spillover_outcome`,
  `admission_enqueued_at_ms`, `admission_queue_wait_ms`,
  `scheduler_woke_at_ms`, `scheduler_poll_ready_entered_at_ms`,
  `scheduler_poll_ready_resolved_at_ms`, `scheduler_poll_ready_wait_ms`,
  `scheduler_dequeued_at_ms`, `scheduler_service_call_started_at_ms`,
  `scheduler_service_call_returned_at_ms`,
  `scheduler_service_call_sync_exec_ms`, and
  `scheduler_ready_to_dispatch_wait_ms`.
- Adds bounded same-file ownership and barrier attribution fields including
  `completion_barrier_active_at_dequeue`, `completion_barrier_generation`,
  `completion_barrier_owner_method`, `completion_barrier_owner_uri`,
  `completion_barrier_owner_version`, `completion_barrier_wait_ms`,
  `doc_sync_first_poll_exec_ms`, `doc_sync_first_poll_outcome`,
  `doc_sync_first_poll_method`, `doc_sync_first_poll_uri`,
  `doc_sync_first_poll_version`, `same_file_ingress_token_required_version`,
  `same_file_ingress_token_published_at_ms`,
  `same_file_ingress_token_source`, and
  `same_file_ingress_token_wait_ms`.

Migration note: timeline consumers must switch to `v22` and expect
`response.version=25`. Tooling that validates, documents, summarizes, or
projects the server-generated payload must read the new bounded pre-dispatch
fields when they are available, continue treating
`response_output_enqueue_completed_at_ms` as legacy writer-selection
compatibility evidence, and degrade explicitly for `v24` payloads where reader
wait, admission, barrier ownership, same-file ingress-token gating, and
residual post-ready decomposition are unavailable by design.
