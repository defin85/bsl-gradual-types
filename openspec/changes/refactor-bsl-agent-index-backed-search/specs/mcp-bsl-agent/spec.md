## ADDED Requirements
### Requirement: MCP type discovery инструменты используют index-backed runtime path (MUST)
`bsl_types_search_start` и parity endpoint `GET /api/mcp/search` MUST выполнять поиск через shared runtime index-backed query path, а не через per-request полный линейный обход всех типов с ad-hoc материализацией workspace state.

Система MUST сохранять детерминированный порядок выдачи в пределах одного `analysis_revision`.

Связанный capability: `bsl-intellisense-v2` (shared runtime search contract).

#### Scenario: MCP type search использует общий индексный путь
- **GIVEN** workspace-сессия `ready=true` и доступен актуальный runtime snapshot
- **WHEN** клиент вызывает `bsl_types_search_start` с валидным `query`
- **THEN** результат формируется через shared runtime index-backed query path
- **AND** повторный вызов с тем же snapshot возвращает тот же порядок результатов

### Requirement: MCP symbol/references batch tools используют candidate-first index path (MUST)
`bsl_symbol_search_start` и `bsl_references_start` MUST использовать candidate-first index path как основной механизм отбора результатов.

Для `bsl_references_start` система MUST поддерживать ограниченную семантическую верификацию только по кандидатам, а не полный workspace scan на каждый запрос.

Primary search path MUST оставаться discovery-oriented: сервер MUST возвращать compact candidate payload и MUST NOT требовать snippet materialization или unconditional full semantic execution для каждого candidate как обязательную часть symbol search path.

#### Scenario: Symbol search не требует полного workspace scan на каждый запрос
- **GIVEN** workspace-сессия `ready=true` и индекс symbols актуален
- **WHEN** клиент вызывает `bsl_symbol_search_start`
- **THEN** сервер отбирает результаты из индексных candidates
- **AND** не запускает per-request полный обход всех BSL-файлов как основной путь
- **AND** не требует snippet materialization или полного semantic pass по каждому candidate как обязательный primary path

#### Scenario: References выполняется как candidate-first с ограниченной верификацией
- **GIVEN** клиент передал валидный `symbol_id` для `bsl_references_start`
- **WHEN** сервер выполняет references query
- **THEN** сервер сначала получает candidates из индексного слоя
- **AND** семантическая проверка выполняется только для candidate subset

### Requirement: MCP search инструменты консистентны с overlays и revision-bound semantics (MUST)
Search-инструменты `bsl_types_search_start`, `bsl_symbol_search_start`, `bsl_references_start` MUST учитывать effective состояние документов (overlay + disk) и MUST соблюдать revision-bound поведение.

Устаревшие batch jobs для поиска MUST завершаться как superseded и MUST NOT публиковать результат как актуальный.

#### Scenario: Overlay изменение supersede-ит устаревший symbol search
- **GIVEN** запущен `bsl_symbol_search_start` на revision `N`
- **AND** клиент выполняет `workspace_documents_set`, переводя сессию на revision `N+1`
- **WHEN** job для revision `N` продолжается
- **THEN** job завершается как superseded
- **AND** актуальным считается только результат для последней revision

### Requirement: MCP search path и fallback причины наблюдаемы и явны (MUST)
Observability для MCP search MUST фиксировать:
- путь выполнения (`index`, `fallback` или `legacy_forced`);
- low-cardinality причину fallback / forced rollback;
- агрегаты candidates/results для triage.

Fallback MUST NOT быть silent: при fallback система MUST давать явный сигнал через observability контракт.

Принудительный rollback на legacy path через operator override MUST также быть явным только в observability и MUST NOT менять публичный `job_result` contract search tools в рамках этого change.

#### Scenario: Fallback path отражается в observability
- **GIVEN** index-backed path недоступен для конкретного search запроса
- **WHEN** сервер завершает запрос через fallback path
- **THEN** observability snapshot содержит `search_path=fallback`
- **AND** содержит low-cardinality fallback reason, достаточную для triage

#### Scenario: Принудительный legacy rollback виден в observability, но не меняет `job_result`
- **GIVEN** оператор включил temporary rollback override для MCP search
- **WHEN** клиент вызывает `bsl_types_search_start`, `bsl_symbol_search_start` или `bsl_references_start`
- **THEN** observability snapshot содержит `search_path=legacy_forced`
- **AND** содержит `fallback_reason=rollout_override`
- **AND** публичный `job_result` не получает дополнительных debug/operator полей только из-за rollout override
