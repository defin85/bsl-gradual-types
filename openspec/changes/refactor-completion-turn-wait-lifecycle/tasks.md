## 1. Runtime lifecycle
- [ ] 1.1 Уточнить и реализовать contract, что same-file completion в состоянии `turn_wait` остаётся discoverable для latest-wins/cancel до active registration и не может стать orphaned waiter.
- [ ] 1.2 Выровнять supersession/cancel propagation между queued, `turn_wait` и active completion states на existing completion path без concurrency-only workaround.
- [ ] 1.3 Сохранить bounded superseded/cancelled terminal outcome и запрет late publish user-facing completion ответа для request, который был остановлен ещё на `turn_wait` lifecycle.

## 2. Observability и contract baseline
- [ ] 2.1 Уточнить authoritative completion timeline contract для truthful `turn_wait` lifecycle: current-request absolute timestamps не должны схлопывать multi-second wait в нулевую длительность, если wait реально наблюдался.
- [ ] 2.2 Обновить versioned contract baseline, incident/export surfaces и graceful degradation для старых payload только там, где это требуется новым `turn_wait` contract.

## 3. Валидация
- [ ] 3.1 Добавить red/green regression для overlapping same-file completion, где older request superseded/cancelled, пока он уже вышел из queue, но ещё не стал active.
- [ ] 3.2 Расширить representative real-module overlap gate и checked-in evidence так, чтобы gate fail-ил на stranded same-file contender в `phase=turn_wait` и на seconds-scale pre-poll backlog нового request.
- [ ] 3.3 Прогнать `openspec validate refactor-completion-turn-wait-lifecycle --strict --no-interactive` и зафиксировать архитектурный review против `refactor-document-symbol-interactive-isolation` и `refactor-completion-superseded-active-turn-release`.

> Зависимости: `2.1` опирается на runtime semantics из `1.1`-`1.3`; `3.2` нельзя закрывать до появления финального overlap evidence. Change остаётся completion-scoped follow-up и не должен превращаться в общий scheduler redesign.
