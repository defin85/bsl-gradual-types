## MODIFIED Requirements

### Requirement: Все runtime tunables `BSL_*` управляемы без рестарта LSP
Система SHALL позволять управлять runtime `BSL_*` параметрами через VS Code settings, и применять их без рестарта LSP процесса.

#### Scenario: Изменение debounce влияет без рестарта
- **GIVEN** LSP сервер запущен
- **WHEN** пользователь меняет настройку, соответствующую `BSL_LSP_DIAGNOSTICS_DEBOUNCE_MS`
- **THEN** последующие diagnostics используют новый debounce без рестарта

