# lsp-index-state v1

## 1.0.0

- Initial baseline for `bsl/getIndexState` startup-orchestration contract.
- Defines state machine values (`idle|running|ready|failed`) and active operations.
- Fixes nullable field policy: `active_operation`, `operation_id`, `message` are always present and use explicit `null` when unset.
- Migration note: initial release, no migration required.
