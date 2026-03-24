# Трассируемость

## Requirement -> Code -> Test

| Requirement | Code | Test / Evidence |
|---|---|---|
| `textDocument/documentSymbol` обслуживается как auxiliary path с bounded outcome-классами и не подменяет strict current-revision semantics interactive ответов | `backend/src/bin/lsp_server/server/language_server/impl_features_a.rs` `backend/src/bin/lsp_server/server/language_server/impl_document_sync.rs` `backend/src/bin/lsp_server/server/core.rs` `backend/src/bin/lsp_server/server/mod.rs` `bsl-runtime/src/system/basic_observability/runtime_metrics.rs` `bsl-runtime/src/system/system_coordinator/coordinator/observability.rs` | `cargo test -p bsl-backend --bin bsl-lsp-server p33_document_symbol_returns_latest_ready_from_cache_during_parse_gap -- --nocapture` `cargo test -p bsl-backend --bin bsl-lsp-server p33_document_symbol_supersedes_older_outstanding_refresh -- --nocapture` |
| Outline burst на том же файле не задерживает первый `poll()` completion и не превращает auxiliary refresh в completion gate | `backend/src/bin/lsp_server/server/language_server/impl_features_a.rs` `backend/src/bin/lsp_server/server/language_server/impl_document_sync.rs` `backend/src/bin/lsp_server/server/core/tests.rs` | `cargo test -p bsl-backend --bin bsl-lsp-server p33_document_symbol_burst_does_not_delay_completion_first_poll_under_parse_gap -- --nocapture` |
| Representative real-module gate fail-closed ловит outline-induced starvation и пишет change-specific checked-in evidence | `backend/src/bin/lsp_server/server/core/tests.rs` `.github/workflows/ci.yml` `scripts/validate-v2-completion-gates.sh` `scripts/README.md` | `cargo test -p bsl-backend --bin bsl-lsp-server p39_real_conf_big_document_symbol_mixed_load_gate_live -- --nocapture` `backend/tests/perf/reports/refactor-document-symbol-interactive-isolation-real-conf-big-document-symbol-mixed-load-live.json` `backend/tests/perf/reports/refactor-document-symbol-interactive-isolation-real-conf-big-document-symbol-mixed-load-live.md` `validation/mixed-load-gate.md` |
| OpenSpec change остаётся синхронизирован с доставленным graph/evidence и проходит strict validation | `openspec/changes/refactor-document-symbol-interactive-isolation/tasks.md` `validation/architectural-review.md` `validation/mixed-load-gate.md` | `openspec validate refactor-document-symbol-interactive-isolation --strict --no-interactive` `backend/tests/perf/reports/refactor-document-symbol-interactive-isolation-openspec-validate.log` |

## OpenSpec / Beads sync
- `tasks.md` отражает delivered state: все пункты `1.x`, `2.x` и `3.x` закрыты.
- Beads graph должен отражать ту же реальность:
  `bsl-gradual-types-7kwl.6`, `bsl-gradual-types-7kwl.7` и `bsl-gradual-types-7kwl.8`
  можно закрывать после появления этих validation artifacts и checked-in reports.
