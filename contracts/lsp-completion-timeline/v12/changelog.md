# lsp-completion-timeline v12

## 12.0.0

- Bumps public response envelope to `version=15`.
- Preserves existing authoritative trace fields and extends the bounded
  `server_edge_details.first_poll_contention_contenders` contract with
  inflight completion stage detail for the pre-first-poll gap:
  - preserves bounded aggregate field-set `contender_class`, `uri_scope`,
    `inflight_count`, `oldest_inflight_age_ms`, `concurrency_level`;
  - preserves bounded-length contender entries with `request_class`, `method`,
    optional `command`, optional `uri`, and `age_ms`;
  - adds optional nested `phase` so bundle consumers can tell whether a stale
    inflight completion is sitting in `prepare_stateful`,
    `wait_exact_type_index`, `query_bundle`, `response_build`, or another
    coarse-grained backend stage.

Migration note: timeline consumers must switch to `v12` and expect
`response.version=15`. Tooling that validates or documents the
server-generated payload must read the expanded `server_edge_details` field
set, keep treating `first_poll_contention_attribution` as the bounded
aggregate fact, and consume `first_poll_contention_contenders` as a bounded
debug snapshot rather than a proof of exact scheduler causality. Consumers
that inspect stale inflight completions should prefer the optional `phase`
field when present and degrade gracefully for `v14` payloads where this detail
is unavailable by design.
