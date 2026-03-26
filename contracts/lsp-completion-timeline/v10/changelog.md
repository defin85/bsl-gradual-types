# lsp-completion-timeline v10

## 10.0.0

- Bumps public response envelope to `version=13`.
- Preserves existing authoritative trace fields and extends the bounded
  `server_edge_details` contract with method-level contender snapshot for the
  pre-first-poll gap:
  - adds optional nested `first_poll_contention_attribution`;
  - adds optional nested `first_poll_contention_contenders`;
  - preserves bounded aggregate field-set `contender_class`, `uri_scope`,
    `inflight_count`, `oldest_inflight_age_ms`, `concurrency_level`;
  - adds bounded-length contender entries with `request_class`, `method`, optional
    `uri`, and `age_ms`.

Migration note: timeline consumers must switch to `v10` and expect
`response.version=13`. Tooling that validates or documents the
server-generated payload must read the expanded `server_edge_details` field set,
treat `first_poll_contention_attribution` as the bounded aggregate fact, and
consume `first_poll_contention_contenders` as a bounded-length debug snapshot
rather than a proof of exact scheduler causality.
