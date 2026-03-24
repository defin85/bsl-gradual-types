# Результат архитектурного review

## Область
- Change: `refactor-document-symbol-interactive-isolation`
- Review focus: граница между auxiliary outline path и interactive semantic path, plus cross-check с уже закрытыми
  `refactor-current-revision-readiness-fast-lane`
  и `refactor-completion-prepare-lightweight-exact-split`
- Дата решения: 24 марта 2026

## Итог
- Change остаётся follow-up после `refactor-current-revision-readiness-fast-lane` и
  `refactor-completion-prepare-lightweight-exact-split`, а не смешивает их scope.
  Producer-side current-revision readiness и lightweight completion prepare остаются отдельными слоями;
  текущий change закрывает только companion-outline starvation рядом с ними.
- `textDocument/documentSymbol` теперь закреплён как auxiliary navigation surface:
  он обслуживается через bounded outcome policy (`current_ready`, `latest_ready`, `unavailable`, `superseded`)
  и не подменяет strict current-revision truth для `completion`, `hover`, `signatureHelp` и `definition`.
- Same-file latest-wins / supersession реализован локально в outline path и не требует detached immutable snapshot architecture.
  Detached-snapshot follow-up остаётся отдельным направлением и не является prerequisite для закрытия этого change.
- Representative real-module gate теперь живёт под собственным `change_id` и явно меряет mixed load
  `didChange`/`didSave` + `documentSymbol` + `completion`, поэтому новый acceptance layer не размазывается по предыдущим change ids.

## Evidence
- Root-cause reasoning и scope:
  `proposal.md`
  `design.md`
- Runtime/LSP path:
  `backend/src/bin/lsp_server/server/language_server/impl_features_a.rs`
  `backend/src/bin/lsp_server/server/language_server/impl_document_sync.rs`
  `backend/src/bin/lsp_server/server/core.rs`
  `backend/src/bin/lsp_server/server/mod.rs`
  `bsl-runtime/src/system/basic_observability/runtime_metrics.rs`
  `bsl-runtime/src/system/system_coordinator/coordinator/observability.rs`
- Regression coverage и representative gate:
  `backend/src/bin/lsp_server/server/core/tests.rs`
  `scripts/validate-v2-completion-gates.sh`
  `.github/workflows/ci.yml`
  `scripts/README.md`
- Checked-in validation artifacts:
  `validation/mixed-load-gate.md`
  `validation/traceability.md`
  `backend/tests/perf/reports/refactor-document-symbol-interactive-isolation-readiness-gate.json`
  `backend/tests/perf/reports/refactor-document-symbol-interactive-isolation-real-conf-big-document-symbol-mixed-load-live.json`

## Остаточные замечания
- Этот review закрывает именно documentSymbol isolation scope.
  Он не утверждает, что все прочие auxiliary LSP methods уже требуют той же обработки.
- Если позже появится новый starvation culprit вне `documentSymbol`, это должен быть отдельный change с новой evidence и собственным representative gate.
