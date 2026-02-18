## ADDED Requirements

### Requirement: `documents_set` обновляет revision и supersede-отмену batch задач (MUST)
Система MUST считать изменения `documents_set` сменой актуальной revision для workspace/session.

Долгие batch semantic job-ы (сканирование файлов/проекта) MUST проверять актуальность revision и завершаться как `superseded`, если пришла более новая revision.

Устаревший batch запуск MUST NOT публиковать результат как актуальный.

#### Scenario: Новая overlay revision отменяет устаревший batch diagnostics job
- **GIVEN** запущен `bsl_diagnostics_start` по `scope=project`
- **AND** в процессе пользователь выполняет `documents_set`, изменяя revision
- **WHEN** batch job продолжает обработку
- **THEN** job завершает устаревший запуск как `superseded`
- **AND** результат устаревшей revision не возвращается как актуальный

### Requirement: MCP tools разделяются на fast interactive и deferred heavy профили (MUST)
Система MUST применять профили выполнения:
- интерактивные инструменты (`bsl_type_at_position`, `bsl_members`, `bsl_definition`) — fast/interactive профиль;
- batch/scanning инструменты (`bsl_diagnostics_start` для project scope, `bsl_symbol_search_start`, `bsl_references_start`) — deferred/background профиль.

Deferred/background профиль MUST NOT вызывать starvation интерактивного пути.

#### Scenario: Интерактивный tool сохраняет прогресс под batch-нагрузкой
- **GIVEN** выполняется долгий `bsl_symbol_search_start`
- **WHEN** параллельно вызывается `bsl_type_at_position`
- **THEN** интерактивный вызов получает runtime слот без ожидания завершения batch хвоста
- **AND** batch продолжает прогресс в background классе

### Requirement: MCP observability использует тот же trigger/profile/reason канон, что и LSP (MUST)
`workspace_get_observability_metrics` в `bsl-agent` MUST публиковать те же low-cardinality semantics для deferred/supersede поведения, что и LSP, с отличием только по `origin=agent`.

Минимально MUST отражаться:
- `trigger` (`documents_set|job_start|idle` где применимо);
- `profile` (`fast|debounced_full|idle_heavy`);
- `reason` (`published|superseded_generation|cancelled` минимум).

Dual-write в MCP MUST оставаться проекцией того же канонического event model без adapter-local переинтерпретации.

#### Scenario: MCP и LSP дают сопоставимый triage по supersede/cancel причинам
- **GIVEN** эквивалентный сценарий устаревания запуска происходит в LSP и в MCP
- **WHEN** сравниваются observability snapshots
- **THEN** причины устаревания и отмены сопоставимы по тем же canonical категориям
- **AND** различие объясняется только `origin`, а не разной семантикой метрик
