## ADDED Requirements

### Requirement: `bsl_diagnostics_start` поддерживает opt-in compact diagnostics mode без breaking changes
Система SHALL поддерживать в `bsl_diagnostics_start` opt-in shaping-параметры для diagnostics payload:
- `compact: bool` (default `false`)
- `group_by: "none" | "message"` (default `"none"`)
- `omit_null_fields: bool` (default `false`)
- `omit_repeated_file: bool` (default `false`)
- `severity_filter: "error" | "warning" | "info"` (optional)

Эти параметры SHALL менять только представление результата, а не сам факт выполнения анализа.

Если `compact=false` и shaping-параметры не заданы, сервер MUST сохранять текущий backward-compatible flat payload (`analysis_revision`, `flow_sensitive_enabled`, `diagnostics[]`, `truncated`).

#### Scenario: Default payload остаётся backward-compatible
- **GIVEN** ready workspace-сессия и вызов `bsl_diagnostics_start` без новых shaping-параметров
- **WHEN** клиент получает `job_result`
- **THEN** сервер возвращает текущий flat payload diagnostics без обязательных compact-only полей

#### Scenario: Compact mode добавляет summary
- **GIVEN** ready workspace-сессия
- **WHEN** клиент вызывает `bsl_diagnostics_start` с `compact=true`
- **THEN** `job_result` содержит `summary.errors`, `summary.warnings`, `summary.infos`, `summary.unique_messages`
- **AND** `summary` описывает именно возвращённую diagnostics выборку

### Requirement: Compact diagnostics mode умеет устранять повторяющийся payload noise
Если `compact=true`, система SHALL поддерживать сокращение повторов в diagnostics payload.

Если `omit_null_fields=true`, nullable поля со значением `null` MUST быть опущены из сериализованного JSON.

Если `omit_repeated_file=true` и все diagnostics результирующей выборки относятся к одному документу, сервер MUST вынести этот документ в top-level `common_file` и MUST NOT дублировать тот же `file` в каждой записи/occurrence.

Если результирующая выборка содержит diagnostics из нескольких файлов, сервер SHALL оставить per-item `file` и SHALL NOT завершаться ошибкой только из-за `omit_repeated_file=true`.

#### Scenario: Single-file diagnostics hoist-ят общий файл
- **GIVEN** ready workspace-сессия и tagged file scope для одного документа
- **WHEN** клиент вызывает `bsl_diagnostics_start` с `compact=true` и `omit_repeated_file=true`
- **THEN** ответ содержит top-level `common_file`
- **AND** per-item diagnostics/occurrences не повторяют тот же `file`

#### Scenario: Null поля не сериализуются в compact mode
- **GIVEN** ready workspace-сессия и diagnostics, где `code` отсутствует
- **WHEN** клиент вызывает `bsl_diagnostics_start` с `compact=true` и `omit_null_fields=true`
- **THEN** в JSON payload поле `code` отсутствует, а не сериализуется как `null`

### Requirement: Compact diagnostics mode поддерживает grouping и severity filtering
Если `compact=true`, система SHALL поддерживать:
- `severity_filter` как server-side фильтр по severity diagnostics;
- `group_by="message"` как deterministic grouped output для повторяющихся diagnostics.

`severity_filter` SHALL применяться до группировки и summary.

Если `group_by="message"`, ответ MUST содержать `groups[]` как primary payload вместо плоского `diagnostics[]`.

Каждая group MUST содержать как минимум:
- `message`
- `severity`
- `count`
- `occurrences[]`

Каждая occurrence MUST содержать данные, достаточные для drilldown, минимум `diagnostic_id` и `range`.

#### Scenario: Group by message сворачивает повторяющиеся diagnostics
- **GIVEN** ready workspace-сессия и несколько diagnostics с одинаковым `message` и `severity`
- **WHEN** клиент вызывает `bsl_diagnostics_start` с `compact=true` и `group_by="message"`
- **THEN** сервер возвращает `groups[]`, где одинаковые сообщения объединены
- **AND** каждая группа содержит `count` и `occurrences[]`
- **AND** flat `diagnostics[]` в ответе не дублируется

#### Scenario: Severity filter оставляет только нужный класс diagnostics
- **GIVEN** ready workspace-сессия и diagnostics разных severity
- **WHEN** клиент вызывает `bsl_diagnostics_start` с `compact=true` и `severity_filter="warning"`
- **THEN** ответ содержит только warning diagnostics / warning groups
- **AND** `summary.errors=0` и `summary.infos=0`
