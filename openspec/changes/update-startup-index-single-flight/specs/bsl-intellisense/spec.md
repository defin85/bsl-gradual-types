## ADDED Requirements

### Requirement: Startup full-index выполняется без дублирования между LSP startup и `bsl/buildIndex` (MUST)
Система MUST обеспечивать single-flight поведение для full-index операций при старте IDE:
- если LSP startup full-index уже выполняется, дополнительный запрос `bsl/buildIndex` MUST NOT запускать второй full-index процесс;
- если full-index уже завершён и состояние `ready`, extension startup MUST NOT инициировать лишний full-index.

#### Scenario: Запрос `bsl/buildIndex` во время startup не запускает второй full-index
- **GIVEN** LSP находится в состоянии startup full-index (`running`)
- **WHEN** extension (или пользователь) вызывает `bsl/buildIndex`
- **THEN** сервер не запускает второй full-index процесс
- **AND** возвращает детерминированный статус, что операция уже выполняется

#### Scenario: После успешного startup extension не запускает повторный full-index
- **GIVEN** LSP сообщает состояние индекса `ready=true`
- **WHEN** extension завершает активацию
- **THEN** extension не инициирует дополнительный full-index на старте

### Requirement: Startup orchestration индекса в extension опирается на server-driven index state (MUST)
VS Code extension MUST принимать решение о запуске full-index на старте по machine-readable состоянию индекса, предоставленному LSP.

Extension MUST NOT использовать локальный filesystem sentinel (`project_indices/.../unified_index.json`) как единственный источник истины для startup решения о full-index.

#### Scenario: Локальный sentinel отсутствует, но сервер уже готов
- **GIVEN** локальный файл sentinel отсутствует или устарел
- **AND** LSP возвращает `ready=true` для index state
- **WHEN** extension выполняет startup orchestration
- **THEN** full-index не запускается повторно только из-за отсутствия sentinel

#### Scenario: Сервер сообщает `failed/idle`, и auto-index включён
- **GIVEN** LSP возвращает `state=failed` или `state=idle`
- **AND** настройка auto-index в extension включена
- **WHEN** extension завершает активацию
- **THEN** extension инициирует один full-index запуск через `bsl/buildIndex`
- **AND** при повторном запросе во время выполнения соблюдается single-flight поведение
