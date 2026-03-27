## 1. Runtime handoff
- [x] 1.1 Спроектировать и реализовать `backend/src/bin/lsp_server/server/transport_adapter.rs` с exported entry point `server::serve_with_completion_handoff(...)`, обновить `backend/src/bin/lsp_server/server/mod.rs` и перевести `backend/src/bin/lsp_server/main.rs` на этот project-local transport/service scheduling adaptation для default event-driven completion path так, чтобы пассивный `turn_wait` не удерживал `tower-lsp` transport slot.
- [x] 1.2 Ввести completion-owned post-handoff lifecycle owner/executor с явным ownership contract для `request_id`, cancellation, shutdown cleanup и ровно одного terminal response.
- [x] 1.3 Сохранить existing same-file latest-wins, explicit cancel и no-late-publish guarantees для queued, handoff-awaiting, `turn_wait` и active completion states.
- [x] 1.4 Сохранить normal LSP response/correlation semantics после handoff: request id, terminal outcome, exactly-once response ownership и fail-closed поведение не должны деградировать.

## 2. Observability и contract
- [x] 2.1 Уточнить authoritative completion timeline / incident export contract так, чтобы off-transport completion wait не маскировался под ingress backlog или handler-resident `turn_wait`, и чтобы handoff/release boundary была видна отдельно.
- [x] 2.2 Обновить versioned contract baseline и graceful degradation для старых payload только там, где это требуется новым handoff contract.
- [x] 2.3 Обновить runbook/agent-facing evidence для scoped transport adaptation и его shutdown/cancel behavior.

## 3. Валидация
- [x] 3.1 Добавить red/green regression для same-file overlap, где current request first-poll bounded, несмотря на multi-second wait за older same-file turn owner уже после handoff.
- [x] 3.2 Добавить targeted regression на race windows `handoff -> cancel/supersede/shutdown`, включая exactly-once terminal response и bounded cleanup.
- [x] 3.3 Расширить representative real-module gate и checked-in evidence так, чтобы gate fail-ил на completion `turn_wait`, который всё ещё удерживает transport slot или превращает handler path в seconds-scale passive wait.
- [x] 3.4 Обновить runbook/CI wiring при необходимости и прогнать `openspec validate refactor-completion-turn-wait-slot-release --strict --no-interactive`.

> Зависимости: `2.1` опирается на runtime handoff из `1.1`-`1.4`; `3.2` нельзя закрывать до финального lifecycle owner contract; `3.3` нельзя закрывать до финального representative evidence. Concrete transport seam для change ограничен `main.rs -> server::serve_with_completion_handoff(...) -> server::transport_adapter`; change остаётся completion-scoped follow-up и не должен расширяться до общего admission-policy redesign.
