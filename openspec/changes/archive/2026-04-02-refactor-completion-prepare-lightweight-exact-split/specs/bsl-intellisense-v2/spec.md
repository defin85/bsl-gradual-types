## ADDED Requirements
### Requirement: Completion first-response prepare разделяет lightweight current-revision path и exact stateful path (MUST)
Для member-access completion система MUST иметь отдельный current-revision prepare contract для first response, не эквивалентный generic heavy `prepare_stateful_operation`.

Этот контракт MUST уметь различать как минимум:
- `head-ready` для current-revision first response;
- `exact-ready` для full exact path;
- bounded `not-ready` для fail-closed path.

Lightweight current-revision prepare MUST:
- быть feature-specific и request-scoped;
- использовать только узкие immutable read-model/DTO данные, необходимые для first completion response;
- MUST NOT публиковать или кэшировать long-lived shared `AnalysisV2` как feature boundary.

#### Scenario: Current-revision head-ready path не требует heavy exact prepare
- **GIVEN** current revision уже имеет queryable `CompletionHeadArtifact`
- **AND** exact semantic path для той же revision еще не ready
- **WHEN** IDE запрашивает member-access completion
- **THEN** completion first response использует lightweight current-revision prepare
- **AND** не требует mandatory full exact stateful prepare как prereq для `head_hit`

#### Scenario: Lightweight prepare fail-closed при отсутствии current-revision truth
- **GIVEN** neither current-revision `CompletionHeadArtifact`, nor exact artifact не ready в пределах bounded policy
- **WHEN** IDE запрашивает member-access completion
- **THEN** completion завершает запрос bounded fail-closed
- **AND** не публикует stale или degraded semantic substitute

## MODIFIED Requirements
### Requirement: LSP interactive операции v2 используют bounded wait + fail-closed freshness policy (MUST)
Для `completion`, `hover`, `signatureHelp` система MUST применять freshness policy:
- сначала пытаться обслужить `requested file version` по фактически `applied_version`;
- ждать не дольше `intellisense_v2_interactive_wait_budget_ms` (дефолт `120ms`, если ключ не задан);
- после исчерпания wait budget завершать запрос fail-closed для текущей revision без stale semantic substitute.

Runtime knob MUST валидироваться и приводиться к диапазону:
- `intellisense_v2_interactive_wait_budget_ms` в диапазон `[10, 2000]`.

Snapshot с несовпадающими `deps_id` или `settings_id`, а также snapshot предыдущей revision, MUST NOT использоваться как semantic substitute для interactive ответа.

Дополнительно для completion:
- completion MUST иметь head-first current-revision prepare path для first response;
- member-access completion MUST NOT требовать generic full `snapshot_with_deps` как обязательный prereq для `head_hit`, если current-revision head truth уже доступен;
- `prepare_stateful_operation` MAY использоваться для completion exact route и exact upgrade, но MUST NOT оставаться обязательной первой ступенью каждого member-access completion после нового `didChange`;
- если `CompletionHeadArtifact` ready внутри wait budget, completion MAY вернуть current-revision semantic response из lightweight head path;
- если `ExactSemanticArtifact` ready внутри wait budget, completion MAY использовать exact semantic response напрямую;
- если внутри wait budget не ready ни один current-revision completion artifact, completion MUST завершиться fail-closed.

#### Scenario: Head-first completion не застревает за heavy generic prepare
- **GIVEN** пользователь только что создал новую requested revision
- **AND** current-revision `CompletionHeadArtifact` уже ready
- **AND** exact semantic path еще не готов
- **WHEN** IDE запрашивает member-access completion
- **THEN** сервер возвращает current-revision first response из head-first path
- **AND** не делает heavy exact prepare обязательной ступенью перед этим ответом

#### Scenario: Exact-only операции остаются на heavy prepare
- **GIVEN** exact semantic artifact текущей revision нужен для `hover`, `definition` или `signatureHelp`
- **WHEN** IDE выполняет такую операцию
- **THEN** сервер использует exact stateful prepare
- **AND** не заменяет его lightweight completion path

### Requirement: Representative real-module gate проверяет current-revision first-response availability для completion (MUST)
Acceptance для архитектурных изменений completion MUST включать representative gate на реальном workspace module, а не только synthetic URI harness.

Этот gate MUST:
- открывать реальный модуль из representative large configuration;
- проверять отдельно `same-revision warm` member-access completion и `revision-churn` completion после нового `didChange` перед каждым measured sample;
- отдельно учитывать first-response availability и exact upgrade latency;
- извлекать route attribution для measured completion samples;
- fail-ить, если measured success path остается effectively heavy exact-first, хотя current-revision head route уже должен быть доступен;
- fail-ить, если completion после новой revision снова деградирует в `fail_closed`, несмотря на наличие current-revision canonical fast path.

#### Scenario: Gate различает lightweight first response и exact upgrade
- **GIVEN** representative real module из большой конфигурации открыт в live gate
- **AND** gate применяет новый `didChange` перед каждым measured completion в `revision-churn` профиле
- **WHEN** выполняется member-access completion
- **THEN** gate требует явную route attribution для measured sample
- **AND** отличает lightweight first response от exact upgrade

#### Scenario: Gate ловит возврат к effectively exact-first completion
- **GIVEN** representative real module и current-revision `CompletionHeadArtifact` уже достижим в пределах bounded policy
- **WHEN** measured completion success все еще зависит от mandatory heavy generic prepare
- **THEN** gate завершает прогон ошибкой
- **AND** отчет выделяет regressions split-prepare boundary отдельно от exact upgrade latency
