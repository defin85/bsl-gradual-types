## ADDED Requirements
### Requirement: LSP document-sync service future освобождает transport slot до slow background стадий (MUST)
`textDocument/didOpen` и `textDocument/didChange` MUST завершать свой service future после того, как:
- входной payload принят;
- `latest_received` и shadow state обновлены для новой requested revision;
- current-revision `SetFile` apply выполнен в analysis runtime для той же `file_version`;
- минимальный handoff slow background work зарегистрирован;
- interactive readers могут видеть новую current revision согласно существующему current-revision contract.

`applied_version` в этом требовании продолжает означать revision, уже применённую в analysis runtime через `SetFile` / `SetFileWithSnapshot`. Она MUST NOT переопределяться как readiness `CompletionHeadArtifact`, `ExactSemanticArtifact` или diagnostics publish.

После этого slow стадии (`parse snapshot build`, current-revision completion precompute, exact precompute, deferred diagnostics) MUST продолжаться вне transport service future.

Document-sync path MUST NOT удерживать LSP transport request-admission slot только ради ожидания завершения этих slow стадий.

#### Scenario: `didChange` освобождает transport slot до завершения parse snapshot
- **GIVEN** changed-text `didChange` для большого модуля запускает дорогой `parse snapshot build`
- **WHEN** LSP принимает notification
- **THEN** document-sync service future завершается после current-revision handoff
- **AND** slow parse snapshot работа продолжается в фоне
- **AND** transport slot не удерживается до терминального завершения parse snapshot

#### Scenario: `didOpen` не ждёт slow parse/head path перед возвратом transport control
- **GIVEN** LSP открывает большой модуль, для которого initial parse/head path дорогой
- **WHEN** сервер принимает `textDocument/didOpen`
- **THEN** document-sync service future завершается после current-revision handoff
- **AND** slow parse/head/exact работа продолжается в фоне
- **AND** initial open не удерживает transport slot до терминального завершения slow path

## MODIFIED Requirements
### Requirement: Completion under large-module churn использует bounded wait и fail-closed current-revision path (MUST)
Для интерактивного completion на больших модулях в состоянии churn система MUST использовать только exact current-revision precomputed artifact (`serve-only`) или явный fail-closed miss для текущей revision.

Completion under churn MUST NOT блокироваться секундными хвостами ожидания latest-path.
Интерактивный request path MUST NOT запускать sync parse/index compute, даже если exact artifact еще недоступен.
Completion under churn MUST NOT накапливать second-scale `service_future_created -> first poll` wait только потому, что более ранние `didOpen/didChange` notifications продолжают slow background стадии после current-revision handoff.

Для этого change `second-scale` pre-poll backlog operationally означает regression, если representative `didChange-burst` gate нарушает budgets, определённые требованием про representative real-module gate.

#### Scenario: Under churn completion отдаёт bounded fail-closed ответ без sync parse/index
- **GIVEN** большой модуль находится в активном churn режиме
- **AND** exact latest artifact временно недоступен в пределах wait budget
- **WHEN** IDE запрашивает completion
- **THEN** сервер возвращает bounded fail-closed response для текущей revision
- **AND** sync parse/index compute не выполняется в интерактивном request path

#### Scenario: Burst document-sync не превращает completion в pre-poll backlog
- **GIVEN** несколько changed-text `didChange` уже перевели файл на новую revision и зарегистрировали slow background работу
- **WHEN** IDE запрашивает member-access completion через live LSP transport path
- **THEN** completion future не проводит seconds-scale время в состоянии "created but not first-polled" только из-за pending document-sync service futures
- **AND** дальнейшая latency атрибуция остаётся отделимой от handler execution

### Requirement: Representative real-module gate проверяет current-revision first-response availability для completion (MUST)
Acceptance для архитектурных изменений completion MUST включать representative gate на реальном workspace module, а не только synthetic URI harness.

Этот gate MUST:
- открывать реальный модуль из representative large configuration;
- проверять отдельно `same-revision warm` member-access completion и `revision-churn` completion после нового `didChange` перед каждым measured sample;
- включать `didChange-burst` профиль через реальный LSP transport path, а не только прямой вызов service layer;
- отдельно учитывать `service_future_to_first_poll_wait_ms`, first-response availability и exact upgrade latency;
- использовать warmup phase, которая не входит в measured set;
- собирать не менее 10 measured completion samples в `didChange-burst` профиле;
- fail-ить, если `p95(service_future_to_first_poll_wait_ms) > 250ms`;
- fail-ить, если любой measured sample имеет `service_future_to_first_poll_wait_ms > 1000ms`, а overshoot атрибутирован pending document-sync futures, а не client-side ingress;
- fail-ить, если completion после новой revision снова деградирует в `fail_closed`, несмотря на наличие current-revision canonical fast path;
- fail-ить, если успешный first response достигается только после seconds-scale pre-poll backlog, вызванного удержанием transport slots document-sync notifications.

#### Scenario: Real-module gate ловит регрессию first-response availability
- **GIVEN** representative real module из большой конфигурации открыт в live gate
- **AND** gate применяет новый `didChange` перед каждым measured completion в `revision-churn` профиле
- **WHEN** выполняется member-access completion
- **THEN** gate требует `ok_non_empty` first response из current-revision canonical artifact
- **AND** gate фиксирует exact upgrade отдельно, не маскируя им first-response availability

#### Scenario: Real-module gate ловит возврат document-sync slot retention
- **GIVEN** gate отправляет burst changed-text notifications через live LSP transport path
- **WHEN** completion timeline показывает seconds-scale `service_future_to_first_poll_wait_ms` до входа в handler
- **THEN** gate завершает прогон ошибкой, даже если completion позже становится `ok_non_empty`
- **AND** отчёт выделяет pre-poll transport backlog отдельно от handler и exact-upgrade latency
