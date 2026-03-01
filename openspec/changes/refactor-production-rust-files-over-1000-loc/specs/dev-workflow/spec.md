## ADDED Requirements

### Requirement: Production Rust source files MUST быть не больше 1000 LOC
Репозиторий MUST поддерживать ограничение размера production Rust исходников: каждый production `.rs` файл MUST быть `<=1000` строк.

Под production scope понимаются `.rs` файлы за исключением:
- `third_party/**`;
- `**/target/**`;
- `**/node_modules/**`;
- тестовых и вспомогательных путей (`tests`, `benches`, `examples`, `fixtures`, `mocks`).

#### Scenario: Size gate блокирует файл больше 1000 строк
- **GIVEN** разработчик изменяет production Rust файл
- **WHEN** запускается size-gate проверка
- **THEN** проверка завершается fail, если файл имеет `>1000 LOC`
- **AND** merge блокируется до декомпозиции файла

### Requirement: Large-file refactor MUST быть behavior-preserving
Кампании декомпозиции крупных production Rust файлов MUST выполняться без изменения наблюдаемого поведения системы.

Это означает:
- публичные контракты (LSP/Web/MCP/CLI) MUST оставаться совместимыми;
- существующие acceptance/regression/perf проверки MUST оставаться неизменными, кроме отдельных согласованных change;
- изменения должны ограничиваться структурой модулей и ответственностей, а не бизнес-семантикой.

#### Scenario: Рефакторинг меняет внешний контракт и отклоняется
- **GIVEN** PR заявлен как large-file behavior-preserving refactor
- **WHEN** contract/regression проверки обнаруживают несовместимое изменение поведения
- **THEN** gate завершается fail
- **AND** merge блокируется до восстановления поведенческой паритетности

### Requirement: Для large-file refactor MUST существовать inventory и parity matrix
Для каждого change, направленного на декомпозицию крупных файлов, MUST быть зафиксированы:
- inventory целевых файлов с исходным размером;
- план batch-декомпозиции с зависимостями;
- parity validation matrix (минимум: compile/lint/tests и релевантные интеграционные проверки).

Progress change MUST измеряться уменьшением inventory до полного отсутствия target файлов `>1000 LOC` в production scope.

#### Scenario: Change без inventory/parity matrix не проходит review gate
- **GIVEN** создан change для декомпозиции крупных файлов
- **WHEN** выполняется pre-implementation review
- **THEN** gate завершается fail, если отсутствует inventory или parity validation matrix
