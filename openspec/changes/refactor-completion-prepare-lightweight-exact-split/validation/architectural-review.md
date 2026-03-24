# Architectural Review Outcome

## Scope
- Change: `refactor-completion-prepare-lightweight-exact-split`
- Review focus: completion first-response boundary, exact-path separation, shipped representative gate coverage
- Decision date: March 24, 2026

## Outcome
- `PreparedCompletionFirstResponse` больше не публикует `AnalysisV2` как внешний carrier.
- Lightweight completion boundary теперь отдаёт узкий request-scoped `CompletionFirstResponseSupport` payload:
  `deps`, `deps_id`, `index_snapshot`, `settings_id`, `file_content`, `file_path`,
  `head_owner_type_hints`, `head_ready`, `exact_ready`.
- Completion request path использует этот payload только для head-first first response; exact route по-прежнему отделён и при необходимости берёт свежий runtime snapshot отдельно.
- Detached immutable snapshot НЕ является prerequisite для closure этого change.
  Текущий delivery закрывает spec за счёт request-scoped immutable DTO на lightweight boundary
  и сохранения `PreparedOperationSnapshot` как canonical heavy exact boundary.

## Evidence
- Narrow public completion boundary:
  [facade.rs](/home/egor/code/bsl-gradual-types/bsl-runtime/src/application/intellisense_v2/facade.rs)
  [operations.rs](/home/egor/code/bsl-gradual-types/bsl-runtime/src/application/intellisense_v2/facade/operations.rs)
  [impl_completion.rs](/home/egor/code/bsl-gradual-types/backend/src/bin/lsp_server/server/language_server/impl_completion.rs)
- Runtime regression coverage for `not-ready` / `head-ready` / `exact-ready` support payload:
  [tests.rs](/home/egor/code/bsl-gradual-types/bsl-runtime/src/application/intellisense_v2/facade/tests.rs)
- Shipped representative gate/evidence for both required real-module profiles:
  [validate-v2-completion-gates.sh](/home/egor/code/bsl-gradual-types/scripts/validate-v2-completion-gates.sh)
  [ci.yml](/home/egor/code/bsl-gradual-types/.github/workflows/ci.yml)
  [README.md](/home/egor/code/bsl-gradual-types/scripts/README.md)
  [readiness-gate.json](/home/egor/code/bsl-gradual-types/backend/tests/perf/reports/refactor-completion-prepare-lightweight-exact-split-readiness-gate.json)
  [warm-cache.json](/home/egor/code/bsl-gradual-types/backend/tests/perf/reports/refactor-completion-prepare-lightweight-exact-split-real-conf-big-warm-cache-completion-perf-live.json)
  [revision-churn.json](/home/egor/code/bsl-gradual-types/backend/tests/perf/reports/refactor-completion-prepare-lightweight-exact-split-real-conf-big-revision-churn-completion-perf-live.json)

## Verification
- `openspec validate refactor-completion-prepare-lightweight-exact-split --strict --no-interactive`
- `cargo test -p bsl-runtime prepare_completion_first_response -- --nocapture`
- `cargo test -p bsl-runtime completion_current_revision_snapshot -- --nocapture`
- `cargo test -p bsl-backend --bin bsl-lsp-server p33_current_revision_head_precompute_stays_available_under_background_cpu_saturation -- --nocapture`
- `cargo test -p bsl-backend --bin bsl-lsp-server p33_completion_head_hit_emits_exact_upgrade_when_background_exact_finishes -- --nocapture`
- `CHANGE_ID=refactor-completion-prepare-lightweight-exact-split ./scripts/validate-v2-completion-gates.sh`

## Residual Notes
- This review closes the architectural question for the current change only.
- Detached immutable completion snapshots remain an optional future evolution, not a blocker for this delivery.
