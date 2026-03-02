## ADDED Requirements
### Requirement: Event-driven precompute `type_index` выполняется на `didOpen/didChange` и публикует version-bound artifacts (MUST)
Система MUST вычислять `type_index` как precompute-артефакт при обработке событий документа (`didOpen`, `didChange`) и MUST связывать результат с ключом:
- `file_id`
- `file_version`
- `deps_id`
- `settings_id`

Система MUST гарантировать latest-wins для одного `file_id`: артефакты superseded версий не должны становиться источником latest serving.

#### Scenario: Burst `didChange` публикует только актуальный precompute artifact
- **GIVEN** для одного файла приходят версии `V`, `V+1`, `V+2` в коротком окне
- **WHEN** precompute jobs выполняются конкурентно/асинхронно
- **THEN** serving latest использует только artifact для актуальной версии
- **AND** artifacts superseded версий не публикуются как latest результат

### Requirement: Интерактивный type lookup использует serve-only cache и не запускает sync parse/index (MUST)
Интерактивные запросы (`completion`, `hover`, `signatureHelp`) MUST получать type lookup данные только из precomputed artifact cache.

В request path MUST NOT запускаться синхронный тяжелый compute `parse_result/type_index` для получения ответа пользователю.

При cache miss система MUST завершать запрос в bounded времени через допустимый degraded outcome (`stale`, `degraded_incomplete`, `fallback_unavailable`) и MUST NOT блокировать ответ длительным пересчетом.

#### Scenario: Cache miss не запускает sync тяжелый compute в интерактивном запросе
- **GIVEN** интерактивный completion запрос пришел до готовности exact artifact
- **WHEN** request path обрабатывает запрос
- **THEN** ответ возвращается в bounded времени с допустимым degraded outcome
- **AND** sync parse/index compute для этого запроса не выполняется

### Requirement: Invalidation артефактов `type_index` детерминирован по `deps_id/settings_id` (MUST)
Система MUST считать artifact невалидным для serving, если изменился `deps_id` или `settings_id` относительно ключа артефакта, даже при совпадении `file_id/file_version`.

#### Scenario: Смена deps invalidates artifact для той же версии файла
- **GIVEN** artifact построен для `(file_id=F, version=V, deps=D1, settings=S1)`
- **AND** runtime переключился на `deps=D2`
- **WHEN** приходит интерактивный запрос для `(F, V)`
- **THEN** artifact `(D1, S1)` не используется как exact latest
- **AND** система использует policy для bounded fallback/recompute по новому ключу

### Requirement: Observability контур различает precompute и serve outcomes low-cardinality метками (MUST)
Система MUST публиковать low-cardinality observability метрики отдельно для:
- precompute queue wait/exec/build;
- serving outcomes (`exact`, `stale`, `degraded_incomplete`, `fallback_unavailable`);
- supersede/cancel причин precompute jobs.

#### Scenario: Root-cause latency виден как precompute lag vs serve miss
- **GIVEN** растет tail latency completion на большом модуле
- **WHEN** анализируется observability snapshot
- **THEN** можно отделить precompute queue lag от serving cache miss path
- **AND** outcome классы serving доступны как low-cardinality counters

## MODIFIED Requirements
### Requirement: Completion under large-module churn использует dual-path bounded fastpath (MUST)
Для интерактивного completion на больших модулях в состоянии churn система MUST применять двухфазную стратегию:
- latest-path через exact precomputed artifact (`serve-only`);
- bounded stale/degraded fallback после исчерпания wait budget или при отсутствии exact artifact.

Completion under churn MUST NOT блокироваться секундными хвостами ожидания latest-path.
Интерактивный request path MUST NOT запускать sync parse/index compute, даже если exact artifact еще недоступен.

#### Scenario: Under churn completion отдаёт bounded ответ без sync parse/index
- **GIVEN** большой модуль находится в активном churn режиме
- **AND** exact latest artifact временно недоступен в пределах wait budget
- **WHEN** IDE запрашивает completion
- **THEN** сервер возвращает bounded stale/degraded ответ
- **AND** sync parse/index compute не выполняется в интерактивном request path
