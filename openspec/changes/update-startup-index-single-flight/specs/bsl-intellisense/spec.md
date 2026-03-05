## ADDED Requirements

### Requirement: Startup full-index выполняется без дублирования между LSP startup и `bsl/buildIndex` (MUST)
Система MUST обеспечивать single-flight поведение для full-index операций при старте IDE:
- если LSP startup full-index уже выполняется, дополнительный запрос `bsl/buildIndex` MUST NOT запускать второй full-index процесс;
- если full-index уже завершён и состояние `ready`, extension startup MUST NOT инициировать лишний full-index.

#### Scenario: Запрос `bsl/buildIndex` во время startup не запускает второй full-index
- **GIVEN** LSP находится в состоянии startup full-index (`running`)
- **WHEN** extension (или пользователь) вызывает `bsl/buildIndex`
- **THEN** сервер не запускает второй full-index процесс
- **AND** возвращает детерминированный attach-статус (`already running`) с идентификатором текущей операции

#### Scenario: После успешного startup extension не запускает повторный full-index
- **GIVEN** LSP сообщает состояние индекса `ready=true`
- **WHEN** extension завершает активацию
- **THEN** extension не инициирует дополнительный full-index на старте

### Requirement: LSP предоставляет machine-readable контракт состояния индекса `bsl/getIndexState` (MUST)
LSP MUST предоставлять custom request `bsl/getIndexState` с contract version `1`, включающим:
- `version`,
- `state` (`idle|running|ready|failed`),
- `ready`,
- `active_operation`,
- `operation_id`,
- `message`,
- `updated_at_ms`.

Клиент MUST использовать этот контракт как источник истины для startup orchestration full-index.

#### Scenario: Клиент получает `running` состояние активной операции
- **GIVEN** сервер выполняет startup full-index
- **WHEN** extension вызывает `bsl/getIndexState`
- **THEN** сервер возвращает `state=running`
- **AND** указывает `active_operation` и `operation_id`

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

#### Scenario: Legacy LSP не поддерживает `bsl/getIndexState`
- **GIVEN** extension подключён к LSP версии без `bsl/getIndexState` (ответ `Method not found`)
- **WHEN** extension выполняет startup orchestration
- **THEN** extension не запускает silent full-index автоматически (fail-closed)
- **AND** показывает явное предупреждение о несовместимости
- **AND** оставляет доступной ручную команду `Build Index`

### Requirement: Running-состояние full-index имеет fail-safe timeout (MUST)
Система MUST иметь watchdog timeout для full-index в состоянии `running`.

При превышении timeout система MUST переводить состояние в `failed` и очищать признак активной операции, чтобы последующий retry мог быть выполнен детерминированно.

#### Scenario: Зависшая операция выходит в `failed` по timeout
- **GIVEN** full-index находится в `running` дольше configured timeout
- **WHEN** watchdog фиксирует превышение лимита
- **THEN** состояние индекса переводится в `failed`
- **AND** `active_operation`/`operation_id` сбрасываются
- **AND** следующий ручной или startup-triggered build может запуститься заново
