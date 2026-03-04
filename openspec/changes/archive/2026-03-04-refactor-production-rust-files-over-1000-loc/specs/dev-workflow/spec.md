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

### Requirement: Large-file target files MUST укладываться в LLM-friendly budget
Для файлов, входящих в inventory large-file refactor change, система MUST применять дополнительный budget, чтобы файл можно было анализировать LLM целиком:
- `LOC <= 800`;
- `bytes <= 80 KiB`;
- `tokens <= 12000` по `o200k_base` токенизации.

Проверка MUST выполняться специальным script-based gate и быть детерминированной.

#### Scenario: Target file превышает токен/байт budget и отклоняется
- **GIVEN** файл входит в target inventory large-file refactor
- **WHEN** выполняется LLM-budget gate
- **THEN** gate завершается fail, если нарушен любой из лимитов (`LOC`, `bytes`, `tokens`)
- **AND** merge блокируется до дальнейшей декомпозиции

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

### Requirement: Inline tests MUST быть вынесены из production Rust файлов
В рамках large-file refactor кампании inline тестовые модули в production `.rs` файлах (например `#[cfg(test)] mod tests`) MUST быть вынесены в отдельные test paths.

#### Scenario: Production файл содержит inline test module и отклоняется
- **GIVEN** PR заявлен как large-file behavior-preserving refactor
- **WHEN** policy gate обнаруживает inline test module в production `.rs`
- **THEN** gate завершается fail
- **AND** merge блокируется до переноса тестов в отдельные test paths
