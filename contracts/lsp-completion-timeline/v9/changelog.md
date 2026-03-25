# lsp-completion-timeline v9

## 9.0.0

- Bumps public response envelope to `version=12`.
- Preserves existing authoritative trace fields and extends the bounded
  `server_edge_details` contract with pre-first-poll contention attribution:
  - adds optional nested `first_poll_contention_attribution`;
  - adds bounded nested field-set `contender_class`, `uri_scope`,
    `inflight_count`, `oldest_inflight_age_ms`, `concurrency_level`;
  - constrains contender semantics to bounded vocabularies for contender class
    and URI scope without request-id, raw-URI, or free-text leakage.

Migration note: timeline consumers must switch to `v9` and expect
`response.version=12`. Tooling that validates or documents the
server-generated payload must read the expanded `server_edge_details` field set
and treat `first_poll_contention_attribution` as a bounded server-visible fact,
not as an exact blocker identity.
