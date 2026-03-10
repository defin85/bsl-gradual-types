# observability-completion-v2 v3

## 3.0.0

- Defines the authoritative public bounded fail-closed taxonomy for interactive semantic surfaces:
  - `missing_canonical_ir`
  - `missing_semantic_index`
  - `superseded_revision`
  - `cancelled`
  - `unavailable_by_contract`
- Collapses public completion outcomes to class-level values:
  - `ok_non_empty`
  - `ok_empty`
  - `fail_closed`
  - `cancelled`
  - `handler_error`
- Introduces shared public fail-closed reason counters keyed by bounded `origin` and `operation`.
- Keeps stale/degraded counters only as anti-rescue guards that must stay zero on authoritative fixtures.

Migration note: dashboards and tooling must stop treating `missing_ir`, `wait_not_ready`,
`fallback_unavailable`, and any `type_index_*` reason vocabulary as authoritative public labels.
Use `intellisense_v2_completion_result_total_fail_closed` for class-level completion outcome and
`intellisense_v2_fail_closed_reason_total_origin_<origin>_operation_<operation>_reason_<reason>`
for bounded public fail-closed reasons.
