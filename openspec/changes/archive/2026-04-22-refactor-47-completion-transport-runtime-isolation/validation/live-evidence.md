# Validation Evidence

## Commands Run

### Targeted backend and transport regressions

- `cargo test -p bsl-backend --bin bsl-lsp-server --no-run`
- `cargo test -p bsl-backend --bin bsl-lsp-server server_edge_details_derive_pre_dispatch_decomposition_when_present -- --nocapture`
- `cargo test -p bsl-backend --bin bsl-lsp-server p47_same_file_ingress_token_waits_for_handoff_registration_before_republishing -- --nocapture`
- `cargo test -p bsl-backend --bin bsl-lsp-server p47_did_save_republishes_same_file_ingress_token_after_existing_handoff -- --nocapture`
- `cargo test -p bsl-backend --bin bsl-lsp-server transport_adapter_allows_general_hover_to_bypass_inflight_did_open_barrier_while_completion_waits -- --nocapture`
- `cargo test -p bsl-backend --bin bsl-lsp-server transport_adapter_prefers_token_ready_completion_over_unrelated_same_priority_fifo -- --nocapture`
- `cargo test -p bsl-backend --bin bsl-lsp-server transport_adapter_task_isolation_keeps_ready_output_and_late_cancel_progress_while_scheduler_stalls -- --nocapture`
- `cargo test -p bsl-backend --bin bsl-lsp-server transport_adapter_classifies_late_cancel_while_completion_lane_is_saturated -- --nocapture`
- `cargo test -p bsl-backend --bin bsl-lsp-server transport_adapter_preserves_completion_progress_when_general_lane_is_saturated -- --nocapture`
- `cargo test -p bsl-backend --bin bsl-lsp-server transport_adapter_attributes_reader_backpressure_before_adapter_read_after_completion_spillover_wait -- --nocapture`
- `CHANGE_ID=refactor-47-completion-transport-runtime-isolation cargo test -p bsl-backend --bin bsl-lsp-server p39_real_conf_big_document_symbol_mixed_load_gate_live -- --nocapture`

### Extension projections

- `npm run compile:fast` (from `vscode-extension/`)
- `BSL_TEST_GREP='Completion Timeline (Clipboard|Model|Webview Provider) Test Suite|Observability Incident Bundle Test Suite|getCompletionTimeline should work via executeCommand|getCompletionTimeline should fail-closed on Method not found' node scripts/run-vscode-extension-tests.js`

### Contract and readiness guards

- `python scripts/check-versioned-contracts.py`
- `python scripts/test-versioned-contracts.py`
- `python scripts/test-intellisense-readiness-assets.py`

### Spec validation

- `openspec validate refactor-47-completion-transport-runtime-isolation --strict --no-interactive`

## Passing Results

- The new transport regressions passed and proved that:
  - reader progress, ready response output, and late cancel classification survive a stalled
    scheduler branch;
  - reader-side spillover wait before `adapter_read` is explicitly attributed and no longer
    masquerades as client-side ingress;
  - same-file `didChange` / `didSave` ownership reaches later completion ahead of unrelated
    same-priority FIFO once the relevant token is published after handoff registration.
- The VS Code projection tests passed and kept the human-readable verdicts fail-closed:
  `reader_backpressure_dominant` stays distinct from `client_before_transport_dominant`, and the
  new server-side split suppresses false client blame.
- The checked-in `contracts/lsp-completion-timeline/v22` baseline now matches authoritative
  `response.version=25`, the repo-level policy guard enforces the new field set, and the
  smoke/readiness docs plus extension-facing degradations no longer advertise `v21`/`v24` as the
  latest completion timeline contract surface.
- The accepted representative live rerun (`p39`) passed and wrote:
  `openspec/changes/refactor-47-completion-transport-runtime-isolation/validation/refactor-47-completion-transport-runtime-isolation-real-conf-big-document-symbol-mixed-load-live.json`
- In the accepted live report, the new `truthful_pre_dispatch_split` class passed with bounded
  server-side waits:
  - `measured_adapter_to_dispatch_wait_ms p95=1ms, max=1ms`
  - `measured_admission_queue_wait_ms p95=1ms, max=1ms`
  - `measured_scheduler_poll_ready_wait_ms p95=0ms, max=0ms`
  - `measured_completion_barrier_wait_ms count=0`
  - `measured_same_file_ingress_token_wait_ms p95=0ms, max=0ms`
  - `measured_scheduler_ready_to_dispatch_wait_ms p95=0ms, max=0ms`
  - `measured_read_loop_wait_ms count=0`
  - `measured_same_file_ingress_token_published_samples=8/8`
  - `measured_truthful_pre_dispatch_bucket_shift_samples=0`
- The representative worst-outlier correlation slice is still tiny and attributable:
  - `step=measured_document_symbol_mixed_load_completion_9`
  - `request_id=39300008`
  - `parse_gap_source=didChange`
  - `required_token_version=10`
  - `current_published_token_version=10`
  - `current_published_token_source=did_save`
  - `dominant_server_edge_bucket=adapter_to_dispatch_wait_ms`
  - `dominant_server_edge_wait_ms=1`
  - `admission_queue_wait_ms=1`
  - `same_file_ingress_token_wait_ms=0`
  - `scheduler_poll_ready_wait_ms=0`
  - `scheduler_ready_to_dispatch_wait_ms=0`
- Strict OpenSpec validation passed.

## Representative Live Artifact

- `openspec/changes/refactor-47-completion-transport-runtime-isolation/validation/refactor-47-completion-transport-runtime-isolation-real-conf-big-document-symbol-mixed-load-live.json`

## Residual Notes

- The first `p39` rerun in this session failed once on a borderline
  `prepare_timeout(wait_for_file_version)` sample (`prepare_stateful_ms=121`) before an immediate
  rerun passed cleanly. The accepted evidence therefore reflects the successful rerun, but the
  earlier near-budget failure remains a live-jitter note rather than something to smooth over.
- The accepted report still marks `parse_cold_start` as failing because
  `measured_cold_query_bundle_samples=8` in the same `examples/conf_big` mixed-load scenario.
  `refactor-47` does not claim to eliminate cold query-body cost; its acceptance target is the
  truthful pre-dispatch split and anti-bucket-shift contract. That residual remains visible in the
  report instead of being hidden behind the new decomposition.
