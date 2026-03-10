# lsp-completion-timeline v3

## 3.0.0

- Replaces legacy completion-visible missing-artifact outcomes with canonical-or-fail-closed timeline semantics.
- Public terminal outcomes are now limited to:
  - `ok_non_empty`
  - `ok_empty`
  - `fail_closed`
  - `cancelled`
  - `superseded`
  - `handler_error`
- Collapses legacy completion-visible outcomes such as `missing_ir`, `wait_not_ready`,
  `fallback_unavailable`, `missing_deps`, `missing_file_*`, and `queue_rejected` into
  the single public `fail_closed` timeline outcome.

Migration note: timeline consumers must stop depending on legacy outcome names that encoded
specific artifact-miss or fallback paths. In `v3`, those states are intentionally represented as
`fail_closed`, while `cancelled` and `superseded` remain distinct control-flow outcomes.
