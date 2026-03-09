## MODIFIED Requirements

### Requirement: IntelliSense v2 обеспечивает IDE‑grade completion по выражениям (MUST)
Система SHALL обеспечивать completion v2, который корректно работает для member access в выражениях и цепочках, включая неполный код:
- `Идентификатор.`
- `Вызов().`
- `Коллекция[...].`
- `(expr).`
- цепочки вида `a.b().c[d].e.`

Syntax extraction для неполного кода MAY использовать parse/syntax helpers, но semantic candidates MUST происходить только из canonical IR snapshot текущей revision и его derived semantic index.

Если canonical semantic artifacts для текущей revision недоступны, completion MUST работать fail-closed и MUST NOT синтезировать semantic candidates из stale cache, keyword fallback или альтернативного inference path.
Система MUST NOT возвращать semantic candidates другой revision под видом exact/current-revision completion ответа.

#### Scenario: Completion на неполном коде использует canonical semantic path
- **GIVEN** пользователь набирает `expr.` и код может быть синтаксически неполным
- **WHEN** IDE запрашивает completion на позиции после `.`
- **THEN** система извлекает receiver-выражение syntax-aware способом
- **AND** semantic candidates читаются только из canonical IR snapshot и derived semantic index текущей revision

#### Scenario: Недоступность canonical artifacts не превращается в semantic fallback
- **GIVEN** для текущей revision canonical IR или derived semantic index ещё недоступны
- **WHEN** IDE запрашивает completion на позиции после `.`
- **THEN** сервер возвращает explicit empty/unavailable fail-closed response для этой revision
- **AND** сервер не возвращает stale, degraded или keyword-only semantic substitute

### Requirement: Инкрементальность и корректность позиций в v2 pipeline (MUST)
Система SHALL обеспечивать согласованность позиций между LSP (UTF-16), внутренними byte offsets и tree-sitter incremental parsing, чтобы completion не использовал semantic truth от другой revision после `didChange`.

Система SHALL гарантировать, что interactive semantic ответы после `didChange` опираются только на canonical artifacts текущей revision или fail-closed для этой revision.

#### Scenario: Первый completion после `didChange` не использует semantic truth предыдущей revision
- **GIVEN** пользователь вводит `expr.` и IDE отправляет `didChange` для новой версии документа
- **WHEN** IDE немедленно отправляет `textDocument/completion` в позиции после `.`
- **THEN** сервер отвечает exact semantic результатом для новой revision или fail-closed response для новой revision
- **AND** не использует stale semantic candidates от предыдущей revision как substitute

### Requirement: v2 pipeline является единственным источником истины для вывода типов (MUST)
Система MUST использовать canonical IR как единственный semantic source of truth для IDE-функций (`completion`, `hover`, `signatureHelp`, `definition`, `diagnostics`, `type-at-position`).

`derived semantic index` MUST быть единственным fast query артефактом для интерактивных semantic запросов и MUST строиться только из canonical IR snapshot.

Legacy-пути вывода типов MUST быть удалены (не поддерживаются), включая parse-result-based semantic inference paths, которые существуют параллельно canonical IR.

#### Scenario: Hover и completion используют canonical IR и derived semantic index
- **GIVEN** пользователь работает в IDE с `.bsl` файлом
- **WHEN** IDE запрашивает hover и completion в одной и той же позиции/контексте
- **THEN** ответы опираются на один canonical IR snapshot и derived semantic index той же revision
- **AND** не используют альтернативные semantic inference пути вне canonical IR contract

## ADDED Requirements

### Requirement: Canonical IR и derived semantic index образуют единый semantic core v2 (MUST)
Система MUST иметь единый semantic core вида `canonical IR -> derived semantic index`.

Canonical IR MUST содержать или однозначно порождать semantic facts, достаточные для:
- owner/member resolution;
- type-at-position;
- completion candidate semantics;
- definition/reference anchors, где требуется semantic ownership;
- diagnostics;
- flow-sensitive overlays через CFG.

`derived semantic index` MUST:
- строиться только из canonical IR snapshot текущей revision;
- быть детерминированной projection того же snapshot;
- не выполнять самостоятельный semantic inference;
- не читать `parse_result.program` как самостоятельный semantic source of truth.

#### Scenario: Один canonical IR snapshot даёт один semantic index для всех consumers
- **GIVEN** построен canonical IR snapshot конкретной revision
- **WHEN** система материализует derived semantic index для interactive queries
- **THEN** индекс является projection того же snapshot
- **AND** все consumers читают semantic facts из одного и того же IR-derived contract

#### Scenario: Derived semantic index не переизобретает semantic truth
- **GIVEN** canonical IR snapshot уже содержит owner/member/type truth для позиции
- **WHEN** derived semantic index строится для этой revision
- **THEN** индекс лишь денормализует lookup для fast queries
- **AND** не вычисляет альтернативный semantic результат из parse tree или отдельной эвристики

### Requirement: Facet-aware semantic identity сохраняется в canonical pipeline (MUST)
Для configuration types canonical IR + derived semantic index MUST сохранять facet-aware semantic identity, необходимую для owner/member/property resolution, hover, diagnostics и module-context bindings.

Этот contract MUST сохранять `active_facet` / `available_facets` или семантически эквивалентное представление.
`derived semantic index` MAY денормализовать facet lookup для fast queries, но MUST NOT сплющивать configuration type до plain metadata/platform type name, если это меняет semantic members, properties или owner behavior.

#### Scenario: ObjectModule explicit binding сохраняет object facet semantics
- **GIVEN** код в `ObjectModule` использует `ЭтотОбъект` или `Объект`
- **WHEN** система выполняет `type-at-position`, `hover` или `members`
- **THEN** semantic result сохраняет object-facet identity owner type текущего модуля
- **AND** member/property lookup использует object semantics, а не manager/reference substitute

#### Scenario: RecordSetModule explicit binding сохраняет recordset facet semantics
- **GIVEN** код в `RecordSetModule` использует `ЭтотОбъект` или `Объект`
- **WHEN** система выполняет `type-at-position`, `hover`, `members` или diagnostics для member access
- **THEN** semantic result сохраняет recordset object facet текущего owner type
- **AND** canonical pipeline не теряет members/properties, зависящие от facet-aware lookup

### Requirement: Semantic fast index отделён от discovery/search read-model (MUST)
Система MUST различать:
- semantic fast index для interactive semantic queries;
- discovery/search read-model (`IndexSnapshot` и эквиваленты) для search/discovery сценариев.

Discovery/search read-model MAY сосуществовать в том же runtime, но MUST NOT быть semantic source of truth для `completion`, `hover`, `signatureHelp`, `definition`, `type-at-position`, `diagnostics`.
Недоступность semantic fast index MUST NOT приводить к backfill через discovery/search read-model.

#### Scenario: Search index не подменяет semantic truth
- **GIVEN** в runtime одновременно существуют canonical IR-derived semantic index и discovery/search index
- **WHEN** IDE запрашивает `hover` или `completion`
- **THEN** semantic ответ строится только из canonical IR и semantic fast index текущей revision
- **AND** наличие search index не меняет semantic contract интерактивного ответа

#### Scenario: Search index не становится rescue path при miss semantic fast index
- **GIVEN** discovery/search index доступен, но semantic fast index текущей revision ещё недоступен
- **WHEN** IDE запрашивает `completion`, `hover` или `definition`
- **THEN** сервер отвечает fail-closed для текущей revision
- **AND** не строит semantic payload из discovery/search read-model

### Requirement: Adapter surfaces не реконструируют semantic truth локально (MUST)
LSP/Web/MCP/CLI surfaces MUST использовать shared semantic runtime contract как единственный semantic read path.

Adapters MAY:
- выполнять syntax/position extraction;
- конвертировать spans/offsets в surface-specific coordinates;
- формировать transport payload.

Adapters MUST NOT:
- реконструировать owner/member/type truth локально из `parse_result`;
- использовать текстовые эвристики как substitute для semantic truth;
- использовать adapter-local caches или precomputed artifacts как stale substitute после смены revision;
- materialize-ить alternate semantic answer при miss canonical artifacts.

#### Scenario: Adapter miss остаётся fail-closed
- **GIVEN** canonical IR или derived semantic index текущей revision недоступны
- **WHEN** любой adapter surface запрашивает `completion`, `hover`, `definition` или `type-at-position`
- **THEN** surface возвращает fail-closed результат согласно своему API contract
- **AND** не строит локальный semantic substitute вне shared runtime path

### Requirement: Canonical semantic queries fail-closed при недоступности артефактов (MUST)
Interactive semantic queries (`completion`, `hover`, `signatureHelp`, `definition`, `type-at-position`) MUST завершаться fail-closed, если canonical IR или derived semantic index текущей revision недоступны.

Fail-closed path MUST NOT:
- использовать stale semantic artifacts как substitute;
- возвращать semantic payload предыдущей revision под видом ответа для текущей revision;
- возвращать keyword fallback как semantic answer;
- запускать альтернативный parse-result-based semantic inference path;
- усиливать semantic truth локальной adapter logic.

Observability MAY фиксировать bounded reason-code недоступности, но не MAY вводить отдельный fallback semantic path.

#### Scenario: Hover miss current revision остаётся fail-closed
- **GIVEN** canonical IR или derived semantic index текущей revision недоступны
- **WHEN** IDE запрашивает hover в позиции с member access
- **THEN** сервер возвращает empty/unavailable hover response
- **AND** не materialize-ит semantic ответ из альтернативного non-IR path

#### Scenario: После didChange stale semantic payload не маскируется под current revision
- **GIVEN** пользователь только что изменил документ и current revision ещё не имеет canonical IR или derived semantic index
- **WHEN** IDE запрашивает `hover`, `type-at-position` или `definition`
- **THEN** сервер отвечает fail-closed для текущей revision
- **AND** не возвращает semantic payload, вычисленный для предыдущей revision, как будто он относится к текущему коду

### Requirement: Fail-closed observability использует bounded reason codes (MUST)
Когда interactive semantic запрос завершается fail-closed, observability MUST фиксировать bounded low-cardinality reason code для текущей revision.

Reason code MUST описывать причину недоступности canonical path и MUST NOT обозначать alternate semantic path как допустимый substitute.
Reason taxonomy MUST оставаться low-cardinality и одинаково интерпретироваться во всех interactive surfaces.

#### Scenario: Miss current revision отражается bounded reason code
- **GIVEN** canonical IR или derived semantic index текущей revision недоступны
- **WHEN** IDE запрашивает `hover` или `completion`
- **THEN** observability фиксирует bounded reason code для fail-closed результата
- **AND** причина не маскирует ответ как stale-but-acceptable semantic path

### Requirement: Interactive latency budget защищается canonical fast path, а не fallback semantics (MUST)
Система MUST удовлетворять согласованным representative latency budgets для interactive semantic queries (`completion`, `hover`, `definition`, `type-at-position`) с использованием canonical IR + derived semantic index.

Если latency budget нарушен, система MUST оптимизировать canonical semantic path и MUST NOT возвращать stale, degraded или discovery-backed semantic substitute как механизм соблюдения latency.

#### Scenario: Latency regression не возвращает legacy semantic rescue path
- **GIVEN** representative interactive fixture показывает превышение согласованного latency budget
- **WHEN** команда исправляет производительность v2 semantic pipeline
- **THEN** исправление оптимизирует canonical IR/derived semantic index path
- **AND** merge-state не вводит stale/degraded/search-backed semantic substitute как perf workaround

### Requirement: Applied-owner bare identifier fallback удалён из v2 semantics (MUST)
Система MUST NOT резолвить bare identifiers в `ObjectModule` / `RecordSetModule` через special applied-owner fallback вне canonical IR semantic binding model.

Если implicit module-context identifier semantics нужны продукту, они MUST быть представлены в canonical IR/binding model и одинаково доступны всем consumers.

#### Scenario: Explicit module-context bindings остаются canonical после удаления fallback
- **GIVEN** код в `ObjectModule` или `RecordSetModule` использует explicit context identifier `ЭтотОбъект` или `Объект`
- **WHEN** система выполняет `type-at-position`, `hover`, `definition`, `members` или diagnostics для member access от этого identifier
- **THEN** `ЭтотОбъект` / `Объект` резолвятся через canonical IR/binding model текущей revision
- **AND** owner/member semantics для такого доступа одинаковы во всех consumers
- **AND** система не зависит от applied-owner bare identifier fallback branch

#### Scenario: Bare identifier без canonical binding остаётся unresolved
- **GIVEN** код в `ObjectModule` или `RecordSetModule` содержит bare identifier, который не имеет canonical binding в текущем snapshot
- **WHEN** система выполняет type-at-position, hover или diagnostics для этого identifier
- **THEN** identifier остаётся unresolved согласно canonical semantic contract
- **AND** система не резолвит его через applied-owner fallback branch
