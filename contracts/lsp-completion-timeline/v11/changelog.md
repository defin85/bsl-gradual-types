# lsp-completion-timeline v11

## 11.0.0

- Bumps public response envelope to `version=14`.
- Preserves existing authoritative trace fields and extends the bounded
  `server_edge_details.first_poll_contention_contenders` contract with
  executeCommand subcommand detail for the pre-first-poll gap:
  - preserves bounded aggregate field-set `contender_class`, `uri_scope`,
    `inflight_count`, `oldest_inflight_age_ms`, `concurrency_level`;
  - preserves bounded-length contender entries with `request_class`, `method`,
    optional `uri`, and `age_ms`;
  - adds optional nested `command` for contenders whose `method` is
    `workspace/executeCommand`, so incident bundles can distinguish
    `bsl.getCompletionTimeline`, `bsl.getObservabilityMetrics`, and other
    executeCommand producers without guessing.

Migration note: timeline consumers must switch to `v11` and expect
`response.version=14`. Tooling that validates or documents the
server-generated payload must read the expanded `server_edge_details` field set,
treat `first_poll_contention_attribution` as the bounded aggregate fact, and
consume `first_poll_contention_contenders` as a bounded-length debug snapshot
rather than a proof of exact scheduler causality. Consumers that inspect
`workspace/executeCommand` contenders should prefer the optional `command`
field when present and degrade gracefully for `v13` payloads where this detail
is unavailable by design.
