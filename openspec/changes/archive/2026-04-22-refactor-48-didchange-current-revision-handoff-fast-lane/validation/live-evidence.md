# Live Evidence

## Representative mixed-load bundle

- Bundle: `/home/egor/code/temp/bsl-observability-incident-2026-04-22T10-25-01Z`
- Captured at: `2026-04-22T10:25:01.243Z`
- Build: `0.4.159` / `git b050f812`
- Fixture: `examples/conf_big`
- Scope: single same-file URI with mixed `didChange` / `didSave` / completion activity

## Why this closes `refactor-48`

This bundle no longer reproduces the `refactor-48` failure mode from
`bsl-observability-incident-2026-04-21T20-16-26Z`.

Representative non-empty completion traces show that the didChange current-revision handoff now
becomes observable before the later completion path needs it:

- `completion-trace-5` (`request_id=75`) finishes in `258ms`, dominated by handler-local
  `collect=254ms`, with:
  - `same_file_ingress_token_required_version=7`
  - `same_file_ingress_token_source=did_change`
  - `same_file_ingress_token_wait_ms=0`
  - `completion_barrier_active_at_dequeue=false`
  - `adapter_to_dispatch_wait_ms=0`
  - `service_future_to_first_poll_wait_ms=0`
- `completion-trace-6` (`request_id=80`) finishes in `11ms`, dominated by `prepare_stateful`,
  with:
  - `same_file_ingress_token_required_version=9`
  - `same_file_ingress_token_source=did_change`
  - `same_file_ingress_token_wait_ms=0`
  - `completion_barrier_active_at_dequeue=false`
  - `adapter_to_dispatch_wait_ms=0`
  - `service_future_to_first_poll_wait_ms=0`

The bundle summary also reports:

- `No didChange parse-snapshot fallback evidence was recorded for this bundle.`

## Interpretation

For `refactor-48`, the important result is that post-edit same-file completion no longer spends
seconds-scale time in `completion_barrier_wait_ms` or `same_file_ingress_token_wait_ms` once
same-file ingress has already been observed.

The same bundle still reveals a different residual in `didSave` heavy follow-up
(`shadow_state` after slow same-version ready-snapshot rebuild), but that is outside the
`refactor-48` contract and was formalized separately in
`refactor-49-save-followup-same-version-ready-snapshot-rebuild-bounding`.
