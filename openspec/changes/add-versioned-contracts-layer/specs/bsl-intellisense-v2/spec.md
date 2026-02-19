## ADDED Requirements

### Requirement: Completion v2 и observability completion имеют versioned contract baseline (MUST)
Система MUST поддерживать versioned contract baseline для интерактивного completion v2 в `contracts/**`.

Baseline MUST покрывать как минимум:
- completion surface: trigger context semantics (`TriggerCharacter`, `Invoked`, `TriggerForIncompleteCompletions`, `None`) и outcome классы (`ok_non_empty`, `ok_empty`, `degraded_incomplete`, `fallback_unavailable`);
- observability surface: trigger mode метрики, parity drift, member-access terminal-empty и fallback_unavailable счётчики.

#### Scenario: Изменение completion semantics требует обновления contract baseline
- **GIVEN** разработчик меняет semantics интерактивного completion v2 или имена/лейблы связанных метрик
- **WHEN** change проходит ревью
- **THEN** соответствующий versioned contract baseline в `contracts/**` обновлён
- **AND** для breaking изменения выполнен major version bump по policy
