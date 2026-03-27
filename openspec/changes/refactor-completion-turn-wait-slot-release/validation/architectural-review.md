# Архитектурная проверка

## Проверенные зависимости change

- `refactor-completion-superseded-active-turn-release`
- `refactor-completion-turn-wait-lifecycle`
- `refactor-current-revision-readiness-fast-lane`

## Вывод

Реализация осталась completion-scoped transport handoff follow-up поверх existing LSP runtime path. Change не превратился ни в workaround через рост transport concurrency, ни в общий scheduler redesign.

## Что именно изменено

- В `backend/src/bin/lsp_server/server/transport_adapter.rs` появился project-local transport/service seam, который handoff-ит default event-driven `textDocument/completion` из bounded ingress path до длительного passive `turn_wait`.
- `backend/src/bin/lsp_server/main.rs` переключён на `server::serve_with_completion_handoff(...)`, поэтому default runtime path теперь использует новый seam без opt-in флагов и без test-only wiring.
- `backend/src/bin/lsp_server/server/request_context.rs`, `backend/src/bin/lsp_server/server/language_server/impl_completion.rs`, `backend/src/bin/lsp_server/server/language_server/helpers.rs` и `backend/src/bin/lsp_server/types.rs` уточняют authoritative timeline contract так, чтобы off-transport wait не маскировался под ingress backlog.
- Representative gates и checked-in readiness bundle для change закреплены через `backend/src/bin/lsp_server/server/core/tests.rs`, `scripts/validate-completion-turn-wait-slot-release.sh`, `scripts/validate-v2-completion-gates.sh`, `.github/workflows/ci.yml`, `docs/agent/verification.md` и `docs/guides/development-workflow.md`.

## Что сознательно не делалось

- Не менялся общий admission policy для всех LSP методов.
- Не увеличивался `tower-lsp` transport concurrency level как замена root-cause fix.
- Не вводился отдельный publish authority вне existing dispatcher/epoch checks.
- Не распространялся handoff contract на `documentSymbol`, hover или другие non-completion methods.

## Риски проверки

- Project-local transport adapter остаётся точкой сопровождения относительно upstream `tower-lsp`; риск ограничен completion-scoped seam и regression coverage.
- Readiness evidence теперь зависит от checked-in summaries, wrapper script и CI upload globs. Если один из этих слоёв снова разойдётся, `scripts/test-intellisense-readiness-assets.py` должен fail-ить до shipped smoke.
