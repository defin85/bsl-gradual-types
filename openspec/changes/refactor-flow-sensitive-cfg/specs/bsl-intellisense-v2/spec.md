## ADDED Requirements

### Requirement: v2 pipeline предоставляет CFG для flow-sensitive анализа (MUST)
Система MUST строить и включать control-flow graph (CFG) в v2 pipeline таким образом, чтобы flow-sensitive анализ (type narrowing / null-safety) мог выполняться на основе v2 snapshot без альтернативных путей построения CFG.

#### Scenario: Flow-sensitive анализ доступен из v2 snapshot
- **GIVEN** пользователь работает с `.bsl` файлом в IDE
- **WHEN** система строит v2 snapshot для hover/completion/diagnostics
- **THEN** CFG присутствует в структуре программы (или доступен через стабильный API v2), и flow-sensitive логика использует этот CFG (без fallback на отдельные CFG/анализаторы вне v2)

### Requirement: В репозитории не существует двусмысленного `ControlFlowGraph` между слоями (MUST)
Система MUST избегать ситуации, когда в разных слоях (IR vs Domain) одновременно существуют разные типы с одинаковым именем `ControlFlowGraph`, если они оба используются/экспортируются как часть публичного API.

#### Scenario: Потребители однозначно импортируют CFG
- **GIVEN** разработчик пишет код, который использует CFG для анализа
- **WHEN** он импортирует `ControlFlowGraph`
- **THEN** импорт однозначен (нет двух разных `ControlFlowGraph` в публичной поверхности), либо используются явно разные имена (`Ir*`/`Domain*`) с понятной семантикой
