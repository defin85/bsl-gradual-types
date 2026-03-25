## ADDED Requirements
### Requirement: Superseded active completion освобождает interactive ownership до завершения stale response-build (MUST)
Если same-file completion request уже успел first-poll-нуться и войти в handler, но затем потерял latest-wins из-за более нового completion request или explicit cancel, система MUST перестать считать его владельцем active interactive completion path не позже ближайшего cooperative cancellation checkpoint после того, как supersession/cancel стал наблюдаемым.

Для этого completion pipeline MUST иметь interruption points, достаточные для prompt release stale active request внутри длинного `response_build` tail. Как минимум bounded interruptible contract MUST покрывать `collect`, `rank`, `format` и publish boundary либо эквивалентную implementation boundary с тем же observable результатом.

Этот contract MUST реализовываться на existing completion path. Новый admission workaround, отдельная transport/admission lane, увеличение concurrency само по себе или общий executor redesign MUST NOT считаться выполнением этого требования без prompt release stale active completion внутри существующего completion pipeline.

Superseded active request MUST NOT удерживать newer same-file completion в seconds-scale `service_future_created -> first poll` wait только потому, что stale `response_build` ещё не полностью завершился.

#### Scenario: Новый same-file completion first-poll-ится, пока старый request boundedly сворачивается
- **GIVEN** completion request `A` для файла уже вошёл в handler и начал тяжёлый `response_build`
- **AND** позже приходит более новый completion request `B` для того же файла
- **WHEN** request `A` теряет latest-wins
- **THEN** request `A` boundedly прекращает stale critical path на ближайшем cooperative checkpoint
- **AND** request `B` достигает first poll в пределах interactive policy, а не после seconds-scale stale tail request `A`

#### Scenario: Superseded response-build не публикует поздний user-facing completion
- **GIVEN** active completion request уже находится внутри `collect` / `rank` / `format`
- **WHEN** request получает explicit cancel или становится superseded более новым same-file request
- **THEN** stale request завершает ответ bounded cancelled/superseded outcome
- **AND** пользовательский completion ответ для этого stale request не публикуется поздно после потери актуальности

## MODIFIED Requirements
### Requirement: Representative real-module gate проверяет current-revision first-response availability для completion (MUST)
Acceptance для архитектурных изменений completion MUST включать representative gate на реальном workspace module, а не только synthetic URI harness.

Этот gate MUST:
- открывать реальный модуль из representative large configuration;
- проверять отдельно `same-revision warm` member-access completion и `revision-churn` completion после нового `didChange` перед каждым measured sample;
- включать `didChange-burst` профиль через реальный LSP transport path, а не только прямой вызов service layer;
- включать overlap profile, в котором новый same-file completion приходит, пока предыдущий completion уже active и ещё не завершил stale path;
- отдельно учитывать `service_future_to_first_poll_wait_ms`, first-response availability и exact upgrade latency;
- использовать warmup phase, которая не входит в measured set;
- собирать не менее 10 measured completion samples в `didChange-burst` профиле;
- fail-ить, если `p95(service_future_to_first_poll_wait_ms) > 250ms`;
- fail-ить, если любой measured sample имеет `service_future_to_first_poll_wait_ms > 1000ms`, а overshoot атрибутирован pending document-sync futures, а не client-side ingress;
- fail-ить, если completion после новой revision снова деградирует в `fail_closed`, несмотря на наличие current-revision canonical fast path;
- fail-ить, если успешный first response достигается только после seconds-scale pre-poll backlog, вызванного удержанием transport slots document-sync notifications;
- fail-ить, если overlap profile показывает, что superseded active completion продолжает удерживать newer same-file completion до первого poll после потери latest-wins.

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

#### Scenario: Real-module gate ловит overlap starvation от superseded active completion
- **GIVEN** gate запускает overlapping same-file completion profile на реальном модуле
- **AND** первый completion уже вошёл в active handler path
- **WHEN** более новый same-file completion приходит до завершения stale path первого request
- **THEN** gate требует bounded cancel/superseded outcome для первого request
- **AND** gate fail-ит, если второй request копит seconds-scale `service_future_to_first_poll_wait_ms` из-за того, что первый request не отпустил active ownership после потери latest-wins
