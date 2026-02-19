## ADDED Requirements

### Requirement: Versioned внешние контракты хранятся в `contracts/**` (MUST)
Система MUST хранить публичные внешние контракты в versioned каталоге `contracts/**`.

Минимальная структура MUST включать:
- surface идентификатор в пути (`contracts/<surface>/...`);
- явную major версию (`contracts/<surface>/v1/...`);
- артефакты контракта в рамках версии (schema и/или эквивалентный формализованный формат + примеры).

#### Scenario: Контракт для внешней поверхности фиксируется как versioned артефакт
- **GIVEN** команда вводит/меняет внешний интерфейс (LSP/Web/MCP/observability labels)
- **WHEN** change подготавливается к merge
- **THEN** в `contracts/**` существует versioned contract артефакт для этой поверхности
- **AND** путь контракта содержит surface и номер major версии

### Requirement: Breaking изменения контракта требуют version bump и migration note (MUST)
Система MUST применять version policy к контрактам:
- breaking change MUST сопровождаться major version bump (`vN -> vN+1`);
- breaking change MUST содержать migration note в change/proposal или contract changelog.

#### Scenario: Breaking контрактный change не проходит без version bump
- **GIVEN** PR меняет contract shape/semantics обратно несовместимым образом
- **WHEN** выполняется контрактная проверка
- **THEN** проверка падает, если major версия не увеличена
- **AND** проверка падает, если отсутствует миграционная заметка
