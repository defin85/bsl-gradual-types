# lsp-completion-timeline v17

## 17.0.0

- Bumps public response envelope to `version=20`.
- Preserves all `v16` authoritative timing fields and fixes canonical
  `query_bundle` attribution around grouped query-body stages:
  `query_bundle_pool_wait`, `query_bundle_deps_and_file_snapshot`,
  `query_bundle_owner_hint`, `query_bundle_ir_query`,
  `query_bundle_ir_retry`, `query_bundle_other`.
- Makes grouped query-body taxonomy the canonical public vocabulary for
  `dominant_stage`, incident/report consumers, and versioned contract
  validation. Legacy aggregate `query_bundle` remains transitional-only and is
  no longer part of the canonical `v20` baseline.
- Requires truthful query-body stage accounting on success, cancel, and fail
  paths so spent time after entering query-body does not disappear into
  `unattributed_overhead`.

Migration note: timeline consumers must switch to `v17` and expect
`response.version=20`. Tooling that validates, documents, or summarizes the
server-generated payload must treat grouped `query_bundle_*` stages as the only
canonical query-body vocabulary, derive user-facing verdicts from those stages,
and degrade explicitly for `v19` payloads where truthful grouped query-body
breakdown is unavailable by design.
