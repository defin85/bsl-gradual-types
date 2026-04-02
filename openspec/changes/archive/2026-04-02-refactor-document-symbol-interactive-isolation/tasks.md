## 1. Runtime Contract
- [x] 1.1 Перевести `textDocument/documentSymbol` на auxiliary serving contract с bounded outcome-классами `current_ready`, `latest_ready` и `unavailable`.
- [x] 1.2 Изолировать admission/execution path для `documentSymbol`, чтобы outstanding outline refresh не мог задерживать первый `poll()` для `completion`, `hover`, `signatureHelp` и `definition`.
- [x] 1.3 Добавить per-file supersession/coalescing для устаревших `documentSymbol` refresh под `didChange`/`didSave` churn.

## 2. Observability And Validation
- [x] 2.1 Добавить observability для auxiliary outline path: outcome/route attribution, supersession и mixed-load correlation с interactive completion wait.
- [x] 2.2 Добавить регрессионные тесты для сценария `didChange`/`didSave` + `documentSymbol` + completion на том же файле.
- [x] 2.3 Добавить representative live gate, который прогоняет real-module mixed load (`documentSymbol` + completion) и fail-ит при outline-induced starvation interactive path.
- [x] 2.4 Обновить validation/runbook artifacts и checked-in evidence для нового mixed-load gate.

## 3. Proposal Hygiene
- [x] 3.1 Прогнать `openspec validate refactor-document-symbol-interactive-isolation --strict --no-interactive`.
- [x] 3.2 Провести архитектурный review change против уже закрытых `refactor-current-revision-readiness-fast-lane` и `refactor-completion-prepare-lightweight-exact-split`, чтобы не смешать documentSymbol isolation с detached-snapshot follow-up.
