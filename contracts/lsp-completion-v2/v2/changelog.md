# lsp-completion-v2 v2

## 2.0.0

- Cutover to canonical-or-fail-closed completion contract for current revision.
- Public completion outcomes are narrowed to transport-visible `ok_non_empty` and `ok_empty`.
- Legacy semantic substitute outcomes `degraded_incomplete` and `fallback_unavailable` are removed from the authoritative public baseline.
- Migration note: consumers must treat an empty completion response as either exact-empty or fail-closed current-revision transport shape; stale/degraded semantic substitute outcomes are no longer contract-stable in `v2`.
