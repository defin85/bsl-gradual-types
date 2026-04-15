# Traceability

## Implementation

- `1.1` Immutable non-member catalogs keyed by deps/settings snapshot:
  `bsl-runtime/src/application/type_system/services/completion_service.rs`
  `bsl-runtime/src/application/type_system/mod.rs`
  `bsl-runtime/src/application/mod.rs`
  `backend/src/bin/lsp_server/handlers/completion.rs`
  `backend/src/bin/lsp_server/server/language_server/impl_completion.rs`
  `backend/src/bin/lsp_server/server/language_server/helpers.rs`
  `cli/src/main.rs`
  `bsl-agent/src/session/manager_semantic_core.rs`
- `1.2` Prefix-aware filtering before request-scoped `Candidate` materialization:
  `bsl-runtime/src/application/type_system/services/completion_service.rs`
  `bsl-runtime/src/application/type_system/services/completion_ranking.rs`
- `1.3` Local/contextual/module-routine collection stays revision-sensitive:
  `bsl-runtime/src/application/type_system/services/completion_service.rs`
  `bsl-runtime/src/application/type_system/services/completion_service/tests.rs`
- `1.4` Collect-stage family attribution exported for operator-facing reports:
  `backend/src/bin/lsp_server/types.rs`
  `backend/src/bin/lsp_server/server/mod.rs`
  `backend/src/bin/lsp_server/server/language_server/impl_completion.rs`
  `backend/src/bin/lsp_server/server/language_server/impl_completion_helpers.rs`
  `backend/src/bin/lsp_server/server/core/tests.rs`
  `openspec/changes/refactor-13-non-member-completion-catalog-precompute/validation/observability-evidence.md`

## Validation

- `2.1` Deterministic cache reuse regression:
  `cargo test -p bsl-runtime completion_non_member_warm_snapshot_reuses_immutable_catalogs -- --nocapture`
- `2.2` Local/context-sensitive correctness regression:
  `cargo test -p bsl-runtime completion_non_member_cache_keeps_local_and_contextual_candidates_stable -- --nocapture`
- `2.3` Representative live evidence:
  `CHANGE_ID=refactor-13-non-member-completion-catalog-precompute cargo test -p bsl-backend --bin bsl-lsp-server p42_real_conf_big_warm_non_member_collect_breakdown_gate_live -- --nocapture`
- `2.4` Existing non-member regressions re-run:
  `cargo test -p bsl-runtime completion_non_member_ -- --nocapture`
  `cargo test -p bsl-backend --bin bsl-lsp-server p33_non_member_form_completion_ages_out_of_shadow_empty_success_window -- --nocapture`
  `cargo test -p bsl-backend --bin bsl-lsp-server p33_aged_non_member_completion_skips_blocking_current_revision_snapshot_reprobe -- --nocapture`
- Default-path wiring closure for reviewed gaps:
  `cargo test -p bsl-cli cli_inline_completion_ -- --nocapture`
  `cargo test -p bsl-agent collect_members_uses_exact_owner_hint_on_default_path -- --nocapture`
- `2.5` Strict OpenSpec validation:
  `openspec validate refactor-13-non-member-completion-catalog-precompute --strict --no-interactive`
