## ADDED Requirements
### Requirement: VS Code extension экспортирует AI-friendly observability incident bundle (MUST)
VS Code extension MUST предоставлять явный user-facing export surface для observability incident handoff в формате bundle, пригодном для внешнего AI/incident анализа.

Этот export bundle MUST:
- собираться extension-side поверх уже существующих observability surface-ов;
- использовать authoritative server timeline только из `bsl.getCompletionTimeline`;
- использовать observability metrics snapshot только из `bsl.getObservabilityMetrics`;
- использовать local client probes только из session-local probe buffer extension;
- включать `summary.md` как краткий human-readable report;
- включать `incident.json` как machine-readable derived report;
- включать raw attachments отдельно от derived report;
- не использовать Output panel dump text как canonical raw source;
- не требовать нового server-side custom request в первой итерации;
- не подменять существующие raw panels и copy/debug flows.

Export bundle MUST явно различать:
- authoritative server trace;
- local-only client probes;
- cumulative metrics snapshot.

#### Scenario: Пользователь экспортирует bundle из observability surface
- **GIVEN** extension подключена к LSP и observability surfaces доступны
- **WHEN** пользователь запускает export incident bundle
- **THEN** extension создаёт bundle с `summary.md` и `incident.json`
- **AND** bundle содержит raw attachments для server timeline, client probes и metrics snapshot
- **AND** summary/report не требуют ручного склеивания текста из нескольких UI панелей

#### Scenario: Raw evidence остаётся отдельным от derived summary
- **GIVEN** extension экспортирует incident bundle
- **WHEN** пользователь или внешний инструмент читает bundle
- **THEN** raw данные completion timeline, client probes и metrics snapshot доступны как отдельные attachments
- **AND** derived summary не подменяет и не перезаписывает raw evidence
- **AND** raw attachments не зависят от truncated Output formatting

### Requirement: Incident bundle деградирует предсказуемо при частичной недоступности данных (MUST)
Export incident bundle MUST завершаться fail-closed по отсутствующим sections, но fail-open для самого handoff flow: bundle может быть частичным, если некоторые источники недоступны, однако он MUST явно фиксировать gaps и MUST NOT выдумывать отсутствующие данные.

Partial export semantics MUST:
- сохранять capture metadata даже при частичной недоступности;
- явно помечать unavailable/unsupported sections в `incident.json` и `summary.md`;
- не реконструировать server trace из client probes или aggregate metrics;
- не подменять missing metrics snapshot последним текстовым dump из Output;
- оставлять raw attachments только для реально полученных sections.

#### Scenario: Legacy LSP не поддерживает `bsl.getCompletionTimeline`
- **GIVEN** connected server не поддерживает `bsl.getCompletionTimeline`
- **WHEN** пользователь запускает export incident bundle
- **THEN** export всё равно создаёт bundle
- **AND** bundle явно помечает server timeline как `unsupported`
- **AND** не пытается реконструировать authoritative server trace из local probes

#### Scenario: Metrics snapshot временно недоступен
- **GIVEN** `bsl.getObservabilityMetrics` временно недоступен или завершился ошибкой
- **WHEN** пользователь запускает export incident bundle
- **THEN** export всё равно создаёт bundle с доступными server timeline и/или client probes
- **AND** `incident.json` и `summary.md` явно фиксируют отсутствие metrics snapshot
- **AND** export не подменяет missing metrics текстом из прошлых Output dumps
