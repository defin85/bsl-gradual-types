## ADDED Requirements
### Requirement: Current-revision readiness fast lane продвигает `applied_version` и `CompletionHeadArtifact` раньше slow enrich path (MUST)
После того как `textDocument/didOpen` или `textDocument/didChange` уже завершил свой transport service future и зарегистрировал current-revision handoff для `file_version=V`, система MUST считать interactive-critical минимумом для этого же `file_id`:
- продвижение `applied_version` до `V` через runtime writer path;
- публикацию и queryability `CompletionHeadArtifact` той же revision `V`.

Этот минимум MUST исполняться по readiness fast lane, который:
- получает приоритет над same-file и older-revision `type_index_precompute`, `ExactSemanticArtifact`, deferred diagnostics и прочими slow background стадиями, не являющимися prerequisite для first current-revision response;
- сохраняет latest-wins и supersession semantics для newest revision;
- MUST NOT публиковать stale semantic truth другой revision под видом current-revision readiness.

Post-handoff lag между registered handoff и observable advance `applied_version` MAY оставаться ненулевым, но completion MUST NOT тратить seconds-scale bounded wait только потому, что latest same-file apply стоит позади low-value background backlog.

`CompletionHeadArtifact` для current revision MUST NOT ждать готовности `ExactSemanticArtifact`, `type_index_precompute` или deferred diagnostics той же revision, если для first current-revision response они не обязательны. Exact upgrade MAY продолжаться в фоне.

#### Scenario: Newest same-file apply не ждёт старый background backlog
- **GIVEN** `didChange` уже зарегистрировал current-revision handoff для `file_version=V+1`
- **AND** в системе ещё выполняется older-revision `type_index_precompute` или diagnostics backlog
- **WHEN** completion запрашивается для `V+1`
- **THEN** runtime продвигает `applied_version` до `V+1` по readiness fast lane
- **AND** latest apply не остаётся ждать терминального завершения older background работы

#### Scenario: Current-revision head становится queryable до exact readiness
- **GIVEN** runtime уже продвинул `applied_version` до current revision `V`
- **AND** `ExactSemanticArtifact` для `V` ещё не ready
- **WHEN** completion запрашивается для той же revision `V`
- **THEN** `CompletionHeadArtifact` current revision остаётся publishable/queryable независимо от exact readiness
- **AND** exact upgrade продолжается в фоне

#### Scenario: Superseded readiness work не блокирует newest revision
- **GIVEN** same-file revision `V` уже имеет in-flight apply/head work
- **AND** приходит более новая revision `V+1`
- **WHEN** readiness scheduler перевыставляет latest work
- **THEN** superseded work для `V` не удерживает fast lane перед `V+1`
- **AND** user-facing readiness для `V+1` получает приоритет latest-wins

## MODIFIED Requirements
### Requirement: Completion under large-module churn использует bounded wait и fail-closed current-revision path (MUST)
Для интерактивного completion на больших модулях в состоянии churn система MUST использовать только exact current-revision precomputed artifact (`serve-only`) или явный fail-closed miss для текущей revision.

Completion under churn MUST NOT блокироваться секундными хвостами ожидания latest-path.
Интерактивный request path MUST NOT запускать sync parse/index compute, даже если exact artifact еще недоступен.
Completion under churn MUST NOT исчерпывать bounded wait на фазе `wait_for_file_version` только потому, что latest same-file apply после document-sync handoff остаётся в очереди позади slow background work.
Completion under churn MUST NOT завершаться `exact_deadline`, если `observed_file_version` уже достиг requested current revision, но `CompletionHeadArtifact` всё ещё отсутствует только потому, что head publish сериализован позади `ExactSemanticArtifact`, `type_index_precompute` или deferred diagnostics.

Для этого change и `prepare_timeout@wait_for_file_version`, и post-apply `head_ready=false` `exact_deadline` считаются regressions current-revision readiness fast lane, а не допустимым bounded fail-closed поведением.

#### Scenario: Under churn completion отдаёт bounded fail-closed ответ без sync parse/index
- **GIVEN** большой модуль находится в активном churn режиме
- **AND** exact latest artifact временно недоступен в пределах wait budget
- **WHEN** IDE запрашивает completion
- **THEN** сервер возвращает bounded fail-closed response для текущей revision
- **AND** sync parse/index compute не выполняется в интерактивном request path

#### Scenario: Post-handoff apply backlog считается regression, а не acceptable miss
- **GIVEN** `didChange` уже зарегистрировал handoff для requested revision `V`
- **AND** completion запрашивается для той же revision `V`
- **WHEN** bounded wait истекает на фазе `wait_for_file_version`, потому что latest same-file apply всё ещё стоит позади background backlog
- **THEN** такой исход считается regression readiness scheduler
- **AND** не считается допустимым fail-closed поведением under churn

#### Scenario: Post-apply отсутствие head считается regression, а не normal exact latency
- **GIVEN** completion уже наблюдает `observed_file_version >= requested current revision`
- **AND** `head_ready=false`, потому что publish `CompletionHeadArtifact` ждёт exact/type-index/deferred diagnostics path
- **WHEN** completion завершает exact wait по deadline
- **THEN** такой исход считается regression head-readiness fast lane
- **AND** не считается допустимой exact-upgrade latency

### Requirement: Representative real-module gate проверяет current-revision first-response availability для completion (MUST)
Acceptance для архитектурных изменений completion MUST включать representative gate на реальном workspace module, а не только synthetic URI harness.

Этот gate MUST:
- открывать реальный модуль из representative large configuration;
- проверять отдельно `same-revision warm` member-access completion и `revision-churn` completion после нового `didChange` перед каждым measured sample;
- включать отдельный профиль `post-handoff readiness`, где changed-text `didChange` и completion выполняются по тому же live LSP path в одной серии;
- отдельно учитывать first-response availability и exact upgrade latency;
- использовать warmup phase, которая не входит в measured set;
- собирать не менее 10 measured completion samples в профиле `post-handoff readiness`;
- извлекать из authoritative payload минимум `wait_for_file_version_runtime.queue_wait_ms`, `min_file_version`, `observed_file_version`, `head_ready_before_wait`, `artifact_poll`;
- fail-ить, если `p95(wait_for_file_version_runtime.queue_wait_ms) > 0.50 * interactive_wait_budget_ms`;
- fail-ить, если любой measured sample имеет `wait_for_file_version_runtime.queue_wait_ms > 4 * interactive_wait_budget_ms`;
- fail-ить, если любой measured sample завершился `prepare_timeout@wait_for_file_version` после same-file current-revision handoff;
- fail-ить, если любой measured sample завершился `exact_deadline` при `artifact_poll.observed_file_version == min_file_version` и `head_ready_before_wait=false`;
- fail-ить, если completion после новой revision снова деградирует в `fail_closed`, несмотря на наличие current-revision canonical fast path;
- fail-ить, если успешный first response достигается только после того, как head publish долго ждал background enrich path и это видно по authoritative readiness fields.

#### Scenario: Real-module gate ловит post-handoff apply backlog
- **GIVEN** gate выполняет профиль `post-handoff readiness` на representative real module
- **AND** перед каждым measured sample отправляется changed-text `didChange`
- **WHEN** measured completion завершает `prepare_timeout@wait_for_file_version`
- **THEN** gate завершает прогон ошибкой
- **AND** отчёт выделяет backlog current-revision apply отдельно от transport ingress

#### Scenario: Real-module gate ловит post-apply head gap
- **GIVEN** gate выполняет completion для current revision через live LSP path
- **AND** authoritative payload показывает `artifact_poll.observed_file_version == min_file_version`
- **AND** `head_ready_before_wait=false`
- **WHEN** completion завершает `exact_deadline`
- **THEN** gate завершает прогон ошибкой
- **AND** отчёт выделяет post-apply head gap отдельно от slow exact-upgrade latency
